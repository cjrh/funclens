# funclens

List longest N functions in a codebase.

## Why?

It turns out that simply using LOC as a measure of "code complexity"
is sufficient. More complex metrics like cyclomatic complexity are often
not worth the effort, see JAY et al. (2009) for an empirical study of 
the relationship between cyclomatic complexity and lines of code. In practice,
long functions are often the most complex and hardest to maintain, so finding
them is a good starting point for refactoring efforts.

Many codebases use multiple languages. I wanted a tool that could find long
functions across all languages in a project.

Reference:

JAY, G. , HALE, J. , SMITH, R. , HALE, D. , KRAFT, N. and WARD, C. (2009) Cyclomatic Complexity and Lines of Code: Empirical Evidence of a Stable Linear Relationship. Journal of Software Engineering and Applications, 2, 137-143. doi: [10.4236/jsea.2009.23020](https://www.scirp.org/journal/paperinformation?paperid=779).

## Install

The easiest install is with
[`cargo binstall`](https://github.com/cargo-bins/cargo-binstall): it
downloads a precompiled binary from the GitHub releases, so there is no
Rust toolchain or compile step. This crate is not published to crates.io,
so point `binstall` at the repository instead of a crate name:

```sh
cargo binstall --git https://github.com/cjrh/funclens funclens
```

`binstall` reads the version from the repo, finds the matching release,
and drops the `funclens` binary on your `PATH`.

## Usage

Pay attention: it automatically finds long functions across different languages.

```
$ funclens .
21 process_order ./api/handlers.py
17 renderCart ./web/cart.ts
10 drain ./worker/queue.go
 8 schedule ./worker/scheduler.rs
 5 deploy ./scripts/deploy.sh
 3 handle ./worker/queue.go
```

Each row is the line count, the function name, and the file it lives in. One
ranking spans every language in the tree, so a Python handler and a TypeScript
component sit in the same list. By default the 20 longest functions are shown.

Pass `--logical` to count only lines that carry code, skipping blank and
comment-only lines (the same idea as `cloc`). Here it drops `process_order`
below the comment-free `renderCart`:

```
$ funclens --logical .
17 renderCart ./web/cart.ts
15 process_order ./api/handlers.py
10 drain ./worker/queue.go
 8 schedule ./worker/scheduler.rs
 5 deploy ./scripts/deploy.sh
 3 handle ./worker/queue.go
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

## Known limits

A few languages do not map a function to a single named node, so their handling
is tricky:

- Haskell reports each equation of a multi-clause function separately.
- OCaml counts a `let` only when it takes a parameter, so plain value bindings
  are left out.
- Swift initializers have no name and are not reported.
- Lua skips anonymous functions assigned to a variable (`x = function() end`).

Anonymous functions with no name to report are skipped across all languages.
