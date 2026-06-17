//! The registry of supported languages.
//!
//! This module is the *only* place that knows anything language-specific.
//! Everything else in the program works in terms of [`Lang`] and is unaware
//! of which languages exist. Supporting a new language means adding one entry
//! to [`build_registry`] — a grammar plus a tree-sitter query that captures
//! function definitions. Nothing else in the codebase needs to change.

use std::sync::OnceLock;

use tree_sitter::{Language, Query};

/// A supported language: its grammar, the query that locates function
/// definitions, and the extensions that select it.
///
/// The query must declare two captures: `@func` (the whole definition, whose
/// row span gives the line count) and `@name` (the identifier reported to the
/// user). [`Lang::new`] panics if either is missing — that is a programming
/// error in the registry, not a runtime condition, so it should never reach a
/// user of a built binary.
pub struct Lang {
    /// File extensions (without the dot) that map to this language.
    pub extensions: &'static [&'static str],
    pub language: Language,
    pub query: Query,
    /// Index of the `@func` capture within `query`.
    pub func_capture: u32,
    /// Index of the `@name` capture within `query`.
    pub name_capture: u32,
}

impl Lang {
    fn new(
        label: &'static str,
        extensions: &'static [&'static str],
        language: Language,
        query_source: &str,
    ) -> Lang {
        let query = Query::new(&language, query_source)
            .unwrap_or_else(|e| panic!("invalid {label} query: {e}"));
        let func_capture = capture_index(&query, "func", label);
        let name_capture = capture_index(&query, "name", label);
        Lang { extensions, language, query, func_capture, name_capture }
    }
}

fn capture_index(query: &Query, name: &str, label: &str) -> u32 {
    query
        .capture_index_for_name(name)
        .unwrap_or_else(|| panic!("{label} query is missing the @{name} capture"))
}

/// Return the language for a file extension, or `None` if unsupported.
///
/// `extension` is the part after the final dot, without the dot, and is
/// matched case-sensitively against the registry.
pub fn for_extension(extension: &str) -> Option<&'static Lang> {
    registry()
        .iter()
        .find(|lang| lang.extensions.contains(&extension))
}

/// All registered languages, built once on first use.
pub fn registry() -> &'static [Lang] {
    static REGISTRY: OnceLock<Vec<Lang>> = OnceLock::new();
    REGISTRY.get_or_init(build_registry)
}

fn build_registry() -> Vec<Lang> {
    vec![
        Lang::new(
            "rust",
            &["rs"],
            tree_sitter_rust::LANGUAGE.into(),
            // Free functions and methods inside `impl` blocks.
            "(function_item name: (identifier) @name) @func",
        ),
        Lang::new(
            "python",
            &["py", "pyi"],
            tree_sitter_python::LANGUAGE.into(),
            // Top-level functions and methods; nested defs are matched too,
            // which is the behaviour we want for a per-function line count.
            "(function_definition name: (identifier) @name) @func",
        ),
        // JavaScript, TypeScript, and TSX share the same function constructs
        // and so the same base query. They differ only in how the grammar names
        // a class field (`field_definition` vs `public_field_definition`), so
        // the arrow-valued field pattern is appended per grammar.
        Lang::new(
            "javascript",
            &["js", "jsx", "mjs", "cjs"],
            tree_sitter_javascript::LANGUAGE.into(),
            &format!("{JS_BASE_QUERY}\n{JS_FIELD_PATTERN}"),
        ),
        Lang::new(
            "typescript",
            &["ts", "mts", "cts"],
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            &format!("{JS_BASE_QUERY}\n{TS_FIELD_PATTERN}"),
        ),
        Lang::new(
            "tsx",
            &["tsx"],
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            &format!("{JS_BASE_QUERY}\n{TS_FIELD_PATTERN}"),
        ),
        Lang::new(
            "go",
            &["go"],
            tree_sitter_go::LANGUAGE.into(),
            "(function_declaration name: (identifier) @name) @func
             (method_declaration name: (field_identifier) @name) @func",
        ),
        Lang::new(
            "c",
            &["c", "h"],
            tree_sitter_c::LANGUAGE.into(),
            C_QUERY,
        ),
        Lang::new(
            "cpp",
            &["cc", "cpp", "cxx", "hpp", "hh", "hxx"],
            tree_sitter_cpp::LANGUAGE.into(),
            CPP_QUERY,
        ),
        Lang::new(
            "java",
            &["java"],
            tree_sitter_java::LANGUAGE.into(),
            "(method_declaration name: (identifier) @name) @func
             (constructor_declaration name: (identifier) @name) @func",
        ),
        Lang::new(
            "ruby",
            &["rb"],
            tree_sitter_ruby::LANGUAGE.into(),
            // `name:` accepts identifiers, operators, and setters alike.
            "(method name: (_) @name) @func
             (singleton_method name: (_) @name) @func",
        ),
        Lang::new(
            "csharp",
            &["cs"],
            tree_sitter_c_sharp::LANGUAGE.into(),
            "(method_declaration name: (identifier) @name) @func
             (constructor_declaration name: (identifier) @name) @func
             (local_function_statement name: (identifier) @name) @func",
        ),
        Lang::new(
            "bash",
            &["sh", "bash"],
            tree_sitter_bash::LANGUAGE.into(),
            "(function_definition name: (word) @name) @func",
        ),
        Lang::new(
            "php",
            &["php"],
            tree_sitter_php::LANGUAGE_PHP.into(),
            "(function_definition name: (name) @name) @func
             (method_declaration name: (name) @name) @func",
        ),
    ]
}

/// JavaScript/TypeScript functions take several shapes. Named declarations and
/// class methods carry their own name; arrow and anonymous function expressions
/// borrow the name of the variable they are assigned to. Class-field arrows are
/// handled by a grammar-specific pattern appended to this base (see above).
const JS_BASE_QUERY: &str = "
    (function_declaration name: (identifier) @name) @func
    (generator_function_declaration name: (identifier) @name) @func
    (method_definition name: (property_identifier) @name) @func
    (variable_declarator
        name: (identifier) @name
        value: [(arrow_function) (function_expression)] @func)
";

const JS_FIELD_PATTERN: &str =
    "(field_definition property: (property_identifier) @name value: (arrow_function) @func)";

const TS_FIELD_PATTERN: &str =
    "(public_field_definition name: (property_identifier) @name value: (arrow_function) @func)";

/// C function definitions, including those returning a pointer (where the name
/// sits one level deeper, under a `pointer_declarator`).
const C_QUERY: &str = "
    (function_definition
        declarator: (function_declarator declarator: (identifier) @name)) @func
    (function_definition
        declarator: (pointer_declarator
            (function_declarator declarator: (identifier) @name))) @func
";

/// C++ adds member functions, qualified names, operators, and destructors, plus
/// pointer- and reference-returning forms.
const CPP_QUERY: &str = "
    (function_definition
        declarator: (function_declarator declarator: [
            (identifier) (field_identifier) (qualified_identifier)
            (destructor_name) (operator_name)
        ] @name)) @func
    (function_definition
        declarator: (pointer_declarator (function_declarator declarator: [
            (identifier) (field_identifier) (qualified_identifier) (operator_name)
        ] @name))) @func
    (function_definition
        declarator: (reference_declarator (function_declarator declarator: [
            (identifier) (field_identifier) (qualified_identifier) (operator_name)
        ] @name))) @func
";
