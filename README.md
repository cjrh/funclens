# funclens

Rank the functions in a project by how many lines they span. Point it at a
directory and it walks the tree, finds every function across 25 languages using
tree-sitter, and prints the longest ones first. Long functions are often where
complexity hides, so this is a quick way to find the parts of a codebase worth a
closer look.

## Install

```
cargo build --release
```

The binary lands at `target/release/funclens`.

## Usage

```
$ funclens src
185 build_registry src/language.rs
 32 extract src/scan.rs
 21 functions_in_file src/scan.rs
 17 collect_code_rows src/scan.rs
 17 main src/main.rs
```

Each row is the line count, the function name, and the file it lives in. By
default the 20 longest functions are shown.

Pass `--logical` to count only lines that carry code, skipping blank and
comment-only lines (the same idea as `cloc`):

```
$ funclens --logical src
163 build_registry src/language.rs
 29 extract src/scan.rs
 19 functions_in_file src/scan.rs
 16 collect_code_rows src/scan.rs
 14 main src/main.rs
```

`.gitignore` is respected, so build artifacts and vendored code stay out of the
results.

## Options

```
funclens [OPTIONS] [PATH]

  PATH                 Directory or file to scan (default ".")
  -n, --number <N>     How many functions to show (default 20)
      --include <EXT>  Only scan these extensions, comma separated, e.g. rs,py
      --exclude <EXT>  Never scan these extensions, comma separated
      --logical        Count lines of code, ignoring blank and comment lines
```

## Supported languages

Bash, C, C#, C++, Dart, Elixir, Go, Haskell, Java, JavaScript, Julia, Kotlin,
Lua, OCaml, Perl, PHP, Python, R, Ruby, Rust, Scala, Swift, TypeScript (and
TSX), Zig.

Adding one means a single entry in `src/language.rs`: a grammar plus a
tree-sitter query that captures function definitions. Nothing else changes.

## Known limits

A few languages do not map a function to a single named node, so their handling
makes a deliberate trade-off:

- Haskell reports each equation of a multi-clause function separately.
- OCaml counts a `let` only when it takes a parameter, so plain value bindings
  are left out.
- Swift initializers have no name and are not reported.
- Lua skips anonymous functions assigned to a variable (`x = function() end`).

Anonymous functions with no name to report are skipped across all languages.
