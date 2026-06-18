//! Walking a directory tree and extracting every function it contains.
//!
//! This module turns a root path into a flat list of [`Function`] records. It
//! hides the file walk, the per-language dispatch, the tree-sitter parsing, and
//! the parallelism behind a single call to [`scan`]. Callers only choose what
//! to look at (the root and an [`ExtFilter`]) and receive ranked-able data.

use std::path::{Path, PathBuf};

use std::collections::HashSet;

use ignore::WalkBuilder;
use rayon::prelude::*;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, QueryCursor};

use crate::language::{self, Lang};

/// How a function's line count is measured.
#[derive(Clone, Copy)]
pub enum CountMode {
    /// Every line the definition spans, blanks and comments included.
    Physical,
    /// Only lines carrying code: blank and comment-only lines are excluded.
    Logical,
}

/// One function definition found in the source tree.
pub struct Function {
    /// The definition's line count under the active [`CountMode`]. A one-line
    /// function counts as 1.
    pub lines: usize,
    pub name: String,
    pub path: PathBuf,
}

/// Selects which file extensions are scanned.
///
/// A file is scanned only if its extension maps to a supported language *and*
/// passes this filter. An empty filter (the default) accepts every supported
/// extension, so the common case requires no configuration.
#[derive(Default)]
pub struct ExtFilter {
    /// If non-empty, only these extensions are scanned.
    pub include: Vec<String>,
    /// These extensions are never scanned; takes precedence over `include`.
    pub exclude: Vec<String>,
}

impl ExtFilter {
    fn accepts(&self, extension: &str) -> bool {
        if self.exclude.iter().any(|e| e == extension) {
            return false;
        }
        self.include.is_empty() || self.include.iter().any(|e| e == extension)
    }
}

/// Find every function under `root`, honouring `.gitignore` and `filter`.
///
/// Unreadable files and files that fail to parse are skipped silently rather
/// than aborting the scan: a single bad file should not deny the user a result
/// for the rest of the tree. The returned order is unspecified; callers rank it.
pub fn scan(root: &Path, filter: &ExtFilter, mode: CountMode) -> Vec<Function> {
    let files = collect_files(root, filter);

    // Rayon pays a fixed thread-pool startup cost. Keep tiny scans on the
    // current thread so `funclens path/to/file.rs` feels instant, while larger
    // trees still benefit from parallel parsing.
    if files.len() < 6 {
        files
            .iter()
            .flat_map(|path| functions_in_file(path, mode))
            .collect()
    } else {
        files
            .par_iter()
            .flat_map_iter(|path| functions_in_file(path, mode).into_iter())
            .collect()
    }
}

/// Gather the paths of all scan-eligible files. The walk itself is cheap and
/// inherently sequential (it reads `.gitignore` state); the expensive parsing
/// is what we parallelise afterwards.
fn collect_files(root: &Path, filter: &ExtFilter) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| eligible(path, filter))
        .collect()
}

fn eligible(path: &Path, filter: &ExtFilter) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => filter.accepts(ext) && language::supports_extension(ext),
        None => false,
    }
}

/// Parse one file and return its functions. Any failure (unreadable file,
/// undecodable bytes, no matching language) yields an empty list.
fn functions_in_file(path: &Path, mode: CountMode) -> Vec<Function> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Vec::new();
    };
    let Some(lang) = language::for_extension(ext) else {
        return Vec::new();
    };
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if parser.set_language(&lang.language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&source, None) else {
        return Vec::new();
    };

    extract(&tree, &source, lang, path, mode)
}

/// Run the language's query over a parsed tree and build [`Function`] records.
fn extract(
    tree: &tree_sitter::Tree,
    source: &str,
    lang: &Lang,
    path: &Path,
    mode: CountMode,
) -> Vec<Function> {
    let src = source.as_bytes();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&lang.query, tree.root_node(), src);
    let mut found = Vec::new();
    // Reused across matches to evaluate any `#eq?`/`#any-of?`/... predicates
    // the query carries (e.g. Elixir filtering `def`/`defp` calls by name).
    let (mut buf1, mut buf2) = (Vec::new(), Vec::new());

    while let Some(m) = matches.next() {
        if !m.satisfies_text_predicates(&lang.query, &mut buf1, &mut buf2, &mut { src }) {
            continue;
        }
        let func = m.nodes_for_capture_index(lang.func_capture).next();
        let name = m.nodes_for_capture_index(lang.name_capture).next();
        let (Some(func), Some(name)) = (func, name) else {
            continue;
        };
        let Ok(name) = name.utf8_text(source.as_bytes()) else {
            continue;
        };
        let lines = count_lines(func, mode);
        found.push(Function {
            lines,
            name: name.to_owned(),
            path: path.to_owned(),
        });
    }
    found
}

/// Count the lines of a function definition under the chosen [`CountMode`].
///
/// Physical counting is the raw row span. Logical counting reports how many of
/// those rows carry code: a row counts if it holds any token that is not a
/// comment, so blank rows and comment-only rows drop out while brace-only rows
/// remain (matching the common `cloc`-style definition).
fn count_lines(func: Node, mode: CountMode) -> usize {
    match mode {
        CountMode::Physical => func.end_position().row - func.start_position().row + 1,
        CountMode::Logical => {
            let mut code_rows = HashSet::new();
            collect_code_rows(func, &mut code_rows);
            code_rows.len()
        }
    }
}

/// Insert into `rows` every source row touched by a code token within `node`.
///
/// Comments are identified grammar-agnostically: most grammars register them as
/// `extra` nodes, and the `kind` check catches the rest. Such nodes — and their
/// subtrees — are skipped, so a comment-only line is never recorded.
fn collect_code_rows(node: Node, rows: &mut HashSet<usize>) {
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        if node.is_extra() || node.kind().contains("comment") {
            continue;
        }
        if node.child_count() == 0 {
            // A leaf token (named or punctuation) marks each row it occupies.
            for row in node.start_position().row..=node.end_position().row {
                rows.insert(row);
            }
        } else {
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
    }
}
