//! The registry of supported languages.
//!
//! This module is the *only* place that knows anything language-specific.
//! Everything else in the program works in terms of [`Lang`] and is unaware
//! of which languages exist. Supporting a new language means adding one entry
//! to [`SPECS`] — a grammar plus a tree-sitter query that captures function
//! definitions. Nothing else in the codebase needs to change.

use std::sync::{LazyLock, OnceLock};

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
    pub language: Language,
    pub query: Query,
    /// Index of the `@func` capture within `query`.
    pub func_capture: u32,
    /// Index of the `@name` capture within `query`.
    pub name_capture: u32,
}

impl Lang {
    fn new(label: &'static str, language: Language, query_source: &str) -> Lang {
        let query = Query::new(&language, query_source)
            .unwrap_or_else(|e| panic!("invalid {label} query: {e}"));
        let func_capture = capture_index(&query, "func", label);
        let name_capture = capture_index(&query, "name", label);
        Lang {
            language,
            query,
            func_capture,
            name_capture,
        }
    }
}

fn capture_index(query: &Query, name: &str, label: &str) -> u32 {
    query
        .capture_index_for_name(name)
        .unwrap_or_else(|| panic!("{label} query is missing the @{name} capture"))
}

/// Declarative registry entry for a language.
///
/// Query compilation is intentionally hidden behind the per-entry [`OnceLock`].
/// Most invocations touch only one or two languages, so eagerly compiling the
/// entire registry makes small scans pay for grammars they will never use.
struct LangSpec {
    /// File extensions (without the dot) that map to this language.
    extensions: &'static [&'static str],
    init: fn() -> Lang,
    lang: OnceLock<Lang>,
}

impl LangSpec {
    const fn new(extensions: &'static [&'static str], init: fn() -> Lang) -> LangSpec {
        LangSpec {
            extensions,
            init,
            lang: OnceLock::new(),
        }
    }

    fn matches(&self, extension: &str) -> bool {
        self.extensions.contains(&extension)
    }

    fn lang(&'static self) -> &'static Lang {
        self.lang.get_or_init(|| (self.init)())
    }
}

/// Return the language for a file extension, or `None` if unsupported.
///
/// `extension` is the part after the final dot, without the dot, and is
/// matched case-sensitively against the registry. The selected language's query
/// is compiled on first use; unrelated language queries are never initialized.
pub fn for_extension(extension: &str) -> Option<&'static Lang> {
    SPECS
        .iter()
        .find(|spec| spec.matches(extension))
        .map(|spec| spec.lang())
}

/// Return whether `extension` maps to a supported language without compiling
/// that language's tree-sitter query.
pub fn supports_extension(extension: &str) -> bool {
    SPECS.iter().any(|spec| spec.matches(extension))
}

static SPECS: LazyLock<Vec<LangSpec>> = LazyLock::new(|| {
    vec![
        LangSpec::new(&["rs"], rust_lang),
        LangSpec::new(&["py", "pyi"], python_lang),
        LangSpec::new(&["js", "jsx", "mjs", "cjs"], javascript_lang),
        LangSpec::new(&["ts", "mts", "cts"], typescript_lang),
        LangSpec::new(&["tsx"], tsx_lang),
        LangSpec::new(&["go"], go_lang),
        LangSpec::new(&["c", "h"], c_lang),
        LangSpec::new(&["cc", "cpp", "cxx", "hpp", "hh", "hxx"], cpp_lang),
        LangSpec::new(&["java"], java_lang),
        LangSpec::new(&["rb"], ruby_lang),
        LangSpec::new(&["cs"], csharp_lang),
        LangSpec::new(&["sh", "bash"], bash_lang),
        LangSpec::new(&["php"], php_lang),
        LangSpec::new(&["zig"], zig_lang),
        LangSpec::new(&["lua"], lua_lang),
        LangSpec::new(&["kt", "kts"], kotlin_lang),
        LangSpec::new(&["swift"], swift_lang),
        LangSpec::new(&["scala", "sc"], scala_lang),
        LangSpec::new(&["ex", "exs"], elixir_lang),
        LangSpec::new(&["ml"], ocaml_lang),
        LangSpec::new(&["hs"], haskell_lang),
        LangSpec::new(&["dart"], dart_lang),
        LangSpec::new(&["jl"], julia_lang),
        LangSpec::new(&["r", "R"], r_lang),
        LangSpec::new(&["pl", "pm"], perl_lang),
    ]
});

fn rust_lang() -> Lang {
    Lang::new(
        "rust",
        tree_sitter_rust::LANGUAGE.into(),
        // Free functions and methods inside `impl` blocks.
        "(function_item name: (identifier) @name) @func",
    )
}

fn python_lang() -> Lang {
    Lang::new(
        "python",
        tree_sitter_python::LANGUAGE.into(),
        // Top-level functions and methods; nested defs are matched too, which
        // is the behaviour we want for a per-function line count.
        "(function_definition name: (identifier) @name) @func",
    )
}

fn javascript_lang() -> Lang {
    Lang::new(
        "javascript",
        tree_sitter_javascript::LANGUAGE.into(),
        &format!("{JS_BASE_QUERY}\n{JS_FIELD_PATTERN}"),
    )
}

fn typescript_lang() -> Lang {
    Lang::new(
        "typescript",
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        &format!("{JS_BASE_QUERY}\n{TS_FIELD_PATTERN}"),
    )
}

fn tsx_lang() -> Lang {
    Lang::new(
        "tsx",
        tree_sitter_typescript::LANGUAGE_TSX.into(),
        &format!("{JS_BASE_QUERY}\n{TS_FIELD_PATTERN}"),
    )
}

fn go_lang() -> Lang {
    Lang::new(
        "go",
        tree_sitter_go::LANGUAGE.into(),
        "(function_declaration name: (identifier) @name) @func
         (method_declaration name: (field_identifier) @name) @func",
    )
}

fn c_lang() -> Lang {
    Lang::new("c", tree_sitter_c::LANGUAGE.into(), C_QUERY)
}

fn cpp_lang() -> Lang {
    Lang::new("cpp", tree_sitter_cpp::LANGUAGE.into(), CPP_QUERY)
}

fn java_lang() -> Lang {
    Lang::new(
        "java",
        tree_sitter_java::LANGUAGE.into(),
        "(method_declaration name: (identifier) @name) @func
         (constructor_declaration name: (identifier) @name) @func",
    )
}

fn ruby_lang() -> Lang {
    Lang::new(
        "ruby",
        tree_sitter_ruby::LANGUAGE.into(),
        // `name:` accepts identifiers, operators, and setters alike.
        "(method name: (_) @name) @func
         (singleton_method name: (_) @name) @func",
    )
}

fn csharp_lang() -> Lang {
    Lang::new(
        "csharp",
        tree_sitter_c_sharp::LANGUAGE.into(),
        "(method_declaration name: (identifier) @name) @func
         (constructor_declaration name: (identifier) @name) @func
         (local_function_statement name: (identifier) @name) @func",
    )
}

fn bash_lang() -> Lang {
    Lang::new(
        "bash",
        tree_sitter_bash::LANGUAGE.into(),
        "(function_definition name: (word) @name) @func",
    )
}

fn php_lang() -> Lang {
    Lang::new(
        "php",
        tree_sitter_php::LANGUAGE_PHP.into(),
        "(function_definition name: (name) @name) @func
         (method_declaration name: (name) @name) @func",
    )
}

fn zig_lang() -> Lang {
    Lang::new(
        "zig",
        tree_sitter_zig::LANGUAGE.into(),
        // One node covers free functions, `pub fn`, and struct methods.
        "(function_declaration name: (identifier) @name) @func",
    )
}

fn lua_lang() -> Lang {
    Lang::new(
        "lua",
        tree_sitter_lua::LANGUAGE.into(),
        // The wildcard name accepts plain, `tbl.method`, and `tbl:method`
        // forms; the assigned-anonymous form (`x = function() end`) is left
        // out, in keeping with the no-anonymous-functions policy.
        "(function_declaration name: (_) @name) @func",
    )
}

fn kotlin_lang() -> Lang {
    Lang::new(
        "kotlin",
        tree_sitter_kotlin_ng::LANGUAGE.into(),
        "(function_declaration name: (identifier) @name) @func",
    )
}

fn swift_lang() -> Lang {
    Lang::new(
        "swift",
        tree_sitter_swift::LANGUAGE.into(),
        // Initializers carry no name node and so are not reported.
        "(function_declaration name: (simple_identifier) @name) @func",
    )
}

fn scala_lang() -> Lang {
    Lang::new(
        "scala",
        tree_sitter_scala::LANGUAGE.into(),
        "(function_definition name: (identifier) @name) @func",
    )
}

fn elixir_lang() -> Lang {
    Lang::new("elixir", tree_sitter_elixir::LANGUAGE.into(), ELIXIR_QUERY)
}

fn ocaml_lang() -> Lang {
    Lang::new(
        "ocaml",
        tree_sitter_ocaml::LANGUAGE_OCAML.into(),
        // A let-binding is a function only if it takes a parameter, which
        // distinguishes `let f x = ...` from a plain `let v = ...` value.
        "(value_definition (let_binding pattern: (value_name) @name (parameter)) @func)",
    )
}

fn haskell_lang() -> Lang {
    Lang::new(
        "haskell",
        tree_sitter_haskell::LANGUAGE.into(),
        // Each equation is its own node, so a multi-clause function is reported
        // once per clause.
        "(function name: (variable) @name) @func",
    )
}

fn dart_lang() -> Lang {
    Lang::new(
        "dart",
        tree_sitter_dart::LANGUAGE.into(),
        "(function_declaration (function_signature name: (identifier) @name)) @func
         (method_declaration
            (method_signature (function_signature name: (identifier) @name))) @func",
    )
}

fn julia_lang() -> Lang {
    Lang::new(
        "julia",
        tree_sitter_julia::LANGUAGE.into(),
        // Long form (`function f(...)`) and assignment form (`f(x) = ...`);
        // the anchor keeps the assignment's left-hand call from matching a call
        // on the right-hand side.
        "(function_definition (signature (call_expression (identifier) @name))) @func
         (assignment . (call_expression (identifier) @name)) @func",
    )
}

fn r_lang() -> Lang {
    Lang::new(
        "r",
        tree_sitter_r::LANGUAGE.into(),
        // Functions are values bound with `<-` or `=`; the name is the
        // left-hand identifier, the line span the function literal.
        "(binary_operator lhs: (identifier) @name rhs: (function_definition) @func)",
    )
}

fn perl_lang() -> Lang {
    Lang::new(
        "perl",
        tree_sitter_perl::LANGUAGE.into(),
        "(function_definition name: (identifier) @name) @func",
    )
}

/// In Elixir a definition is a macro call whose target is `def`/`defp` (etc.),
/// structurally identical to `if`/`for`/`with` blocks. The `#any-of?` predicate
/// is what separates them; the head is either a call (`def f(a) do`) or a bare
/// identifier (`def f do`).
const ELIXIR_QUERY: &str = r#"
    ((call
        target: (identifier) @kw
        (arguments [
            (call target: (identifier) @name)
            (identifier) @name
        ])
        (do_block)) @func
     (#any-of? @kw "def" "defp" "defmacro" "defmacrop"))
"#;

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
