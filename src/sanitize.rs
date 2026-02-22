//! Source-aware sanitizer using `nom`.
//!
//! Strips string literals and comments from source code so that the
//! brace-balance checker only sees *structural* paired characters and
//! never false-positives on braces inside strings or comments.
//!
//! The sanitizer is **deterministic** — it is a single-pass left-to-right
//! parser that replaces every non-structural character inside a string or
//! comment with a space (preserving line count and column offsets).
//!
//! ## Design
//!
//! We do NOT try to be a full lexer for every language.  Instead we
//! configure a set of *comment styles* and *string styles* per language
//! (keyed on file extension) and combine them into a single nom parser.
//!
//! The parser emits a new `String` of the same length where every character
//! inside a string literal or comment is replaced with a space, **except**
//! for newlines which are kept so line numbers stay correct.

use std::path::Path;

// ---------------------------------------------------------------------------
// Per-language syntax configuration
// ---------------------------------------------------------------------------

/// Comment syntax variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    /// `// ...` to end of line
    CLineComment,
    /// `/* ... */`
    CBlockComment,
    /// `# ...` to end of line
    Hash,
    /// `-- ...` to end of line
    DoubleDash,
    /// `{- ... -}`
    HaskellBlock,
    /// `(* ... *)`
    OcamlBlock,
    /// `<!-- ... -->`
    HtmlBlock,
    /// `; ...` to end of line  (Lisp/Clojure)
    Semicolon,
    /// `" ...` to end of line  (Vimscript)
    VimComment,
    /// `% ...` to end of line  (Erlang)
    Percent,
}

/// String literal variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringStyle {
    /// `"..."` with `\"` escape
    Double,
    /// `'...'` with `\'` escape
    Single,
    /// `` `...` `` with `` \` `` escape (JS/TS template literals)
    Backtick,
    /// `"""..."""` Python triple-double
    TripleDouble,
    /// `'''...'''` Python triple-single
    TripleSingle,
    /// `r"..."` Python raw string (double)
    PythonRawDouble,
    /// `r'...'` Python raw string (single)
    PythonRawSingle,
    /// `r#"..."#` Rust raw string (we handle r, r#, r##, etc.)
    RustRaw,
}

/// The combined syntax config for a language.
#[derive(Debug, Clone)]
pub struct LangSyntax {
    pub comments: &'static [CommentStyle],
    pub strings: &'static [StringStyle],
}

// ---------------------------------------------------------------------------
// Static syntax tables
// ---------------------------------------------------------------------------

use CommentStyle::*;
use StringStyle::*;

static C_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[Double, Single],
};

static C_STYLE_BACKTICK: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[Double, Single, Backtick],
};

static PYTHON: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[TripleDouble, TripleSingle, Double, Single, PythonRawDouble, PythonRawSingle],
};

static RUST: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[RustRaw, Double],
};

static HASH_ONLY: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

static HASKELL: LangSyntax = LangSyntax {
    comments: &[DoubleDash, HaskellBlock],
    strings: &[Double, Single],
};

static OCAML: LangSyntax = LangSyntax {
    comments: &[OcamlBlock],
    strings: &[Double],
};

static FSHARP: LangSyntax = LangSyntax {
    comments: &[CLineComment, OcamlBlock],
    strings: &[Double],
};

static HTML: LangSyntax = LangSyntax {
    comments: &[HtmlBlock],
    strings: &[Double, Single],
};

static SQL_STYLE: LangSyntax = LangSyntax {
    comments: &[DoubleDash, CBlockComment],
    strings: &[Single],
};

static LISP: LangSyntax = LangSyntax {
    comments: &[Semicolon],
    strings: &[Double],
};

static ERLANG: LangSyntax = LangSyntax {
    comments: &[Percent],
    strings: &[Double],
};

static VIM: LangSyntax = LangSyntax {
    comments: &[VimComment],
    strings: &[Double, Single],
};

static ELM: LangSyntax = LangSyntax {
    comments: &[DoubleDash, HaskellBlock],
    strings: &[Double],
};

static ELIXIR: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[TripleDouble, Double, Single],
};

static PHP: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment, Hash],
    strings: &[Double, Single],
};

static LUA: LangSyntax = LangSyntax {
    comments: &[DoubleDash],
    strings: &[Double, Single],
};

/// JSON has no comments but has strings that could contain braces.
static JSON_STYLE: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[Double],
};

/// JSONC has C-line comments + strings.
static JSONC_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[Double],
};

/// TOML: `#` comments, basic strings `"`, literal strings `'`
static TOML_STYLE: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

/// YAML: `#` comments, both quote styles
static YAML_STYLE: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

/// Groovy: C-style comments + triple-double strings + single + double
static GROOVY: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

/// Scala: C-style comments + triple-double strings + single + double
static SCALA: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

/// Kotlin: C-style comments + backtick (for identifiers) + double + single (char)
static KOTLIN: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

/// PowerShell: `#` line comment, `<# ... #>` block comment — we approximate
/// block `<# #>` via a dedicated handler below, but for now we use Hash.
static POWERSHELL: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

/// Emacs Lisp: `;` comments
static EMACS_LISP: LangSyntax = LangSyntax {
    comments: &[Semicolon],
    strings: &[Double],
};

/// No-op fallback — no stripping at all.
static PLAIN: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[],
};

/// Look up the syntax configuration for a file based on its extension.
pub fn syntax_for_extension(ext: &str) -> &'static LangSyntax {
    match ext {
        // C-style with backtick template literals
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => &C_STYLE_BACKTICK,

        // C-style (no backtick)
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx"
        | "java" | "go" | "swift" | "dart" | "cs"
        | "proto" | "tf" | "hcl" => &C_STYLE,

        // JVM variants with triple-quoted strings
        "kt" | "kts" => &KOTLIN,
        "scala" => &SCALA,
        "groovy" => &GROOVY,

        // Rust
        "rs" => &RUST,

        // Python
        "py" => &PYTHON,

        // Ruby / Shell / R / Perl / CMake
        "rb" | "sh" | "bash" | "zsh" | "fish" | "r" | "pl" | "pm" | "cmake" | "dockerfile" => &HASH_ONLY,

        // PHP (has //, /* */, and #)
        "php" => &PHP,

        // Lua
        "lua" => &LUA,

        // Elixir
        "ex" | "exs" => &ELIXIR,

        // Erlang
        "erl" | "hrl" => &ERLANG,

        // Haskell
        "hs" => &HASKELL,

        // Elm
        "elm" => &ELM,

        // OCaml
        "ml" | "mli" => &OCAML,

        // F#
        "fs" | "fsi" | "fsx" => &FSHARP,

        // Clojure (Lisp)
        "clj" | "cljs" | "cljc" => &LISP,

        // SQL / GraphQL
        "sql" | "graphql" | "gql" => &SQL_STYLE,

        // HTML / XML
        "html" | "htm" | "xml" => &HTML,

        // JSON
        "json" => &JSON_STYLE,
        "jsonc" => &JSONC_STYLE,

        // TOML
        "toml" => &TOML_STYLE,

        // YAML
        "yml" | "yaml" => &YAML_STYLE,

        // Vim
        "vim" => &VIM,

        // Emacs Lisp
        "el" => &EMACS_LISP,

        // PowerShell
        "ps1" | "psm1" => &POWERSHELL,

        _ => &PLAIN,
    }
}

/// Convenience: get syntax from a file path.
pub fn syntax_for_path(path: &Path) -> &'static LangSyntax {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    syntax_for_extension(&ext)
}

// ---------------------------------------------------------------------------
// The sanitizer — replaces string/comment interior with spaces
// ---------------------------------------------------------------------------

/// Sanitize source code: replace the interior of all string literals and
/// comments with spaces (preserving newlines).  The returned string has the
/// same length and line structure as the input.
///
/// `syntax` determines which comment and string styles are recognised.
pub fn sanitize(input: &str, syntax: &LangSyntax) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        // --- Try each comment style (longest-match first is guaranteed by
        //     the order we try them — multi-char delimiters before single). ---

        if let Some(advance) = try_comment(bytes, i, syntax) {
            // Replace the entire comment span with spaces, keeping newlines.
            blank_preserving_newlines(&bytes[i..i + advance], &mut out);
            i += advance;
            continue;
        }

        // --- Try each string style ---

        if let Some(advance) = try_string(bytes, i, syntax) {
            blank_preserving_newlines(&bytes[i..i + advance], &mut out);
            i += advance;
            continue;
        }

        // --- Regular character: keep as-is ---
        out.push(bytes[i]);
        i += 1;
    }

    // SAFETY: we only replaced ASCII non-newline bytes with spaces and kept
    // multi-byte sequences that are not inside strings/comments verbatim.
    // However there's a subtlety: inside strings/comments we blank ALL bytes
    // including interior multi-byte chars.  Since we only replace with b' '
    // (ASCII space) and keep b'\n', the result is always valid UTF-8 as long
    // as the input was valid UTF-8.  If a multi-byte char straddles a
    // boundary that shouldn't happen because our parsers always consume full
    // delimiter sequences.
    String::from_utf8(out).expect("sanitize produced invalid UTF-8")
}

// ---------------------------------------------------------------------------
// Comment matchers
// ---------------------------------------------------------------------------

/// Try to match a comment starting at position `i`.
/// Returns `Some(byte_count_consumed)` or `None`.
fn try_comment(bytes: &[u8], i: usize, syntax: &LangSyntax) -> Option<usize> {
    for style in syntax.comments {
        if let Some(n) = match_comment(bytes, i, *style) {
            return Some(n);
        }
    }
    None
}

fn match_comment(bytes: &[u8], i: usize, style: CommentStyle) -> Option<usize> {
    let remaining = bytes.len() - i;
    match style {
        CLineComment => {
            if remaining >= 2 && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                Some(consume_to_eol(bytes, i))
            } else {
                None
            }
        }
        CBlockComment => {
            if remaining >= 2 && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                Some(consume_block(bytes, i + 2, b"*/") + 2)
            } else {
                None
            }
        }
        Hash => {
            if bytes[i] == b'#' {
                Some(consume_to_eol(bytes, i))
            } else {
                None
            }
        }
        DoubleDash => {
            if remaining >= 2 && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                Some(consume_to_eol(bytes, i))
            } else {
                None
            }
        }
        HaskellBlock => {
            if remaining >= 2 && bytes[i] == b'{' && bytes[i + 1] == b'-' {
                Some(consume_block(bytes, i + 2, b"-}") + 2)
            } else {
                None
            }
        }
        OcamlBlock => {
            if remaining >= 2 && bytes[i] == b'(' && bytes[i + 1] == b'*' {
                Some(consume_block(bytes, i + 2, b"*)") + 2)
            } else {
                None
            }
        }
        HtmlBlock => {
            if remaining >= 4 && &bytes[i..i + 4] == b"<!--" {
                Some(consume_block(bytes, i + 4, b"-->") + 4)
            } else {
                None
            }
        }
        Semicolon => {
            if bytes[i] == b';' {
                Some(consume_to_eol(bytes, i))
            } else {
                None
            }
        }
        VimComment => {
            // In Vimscript `"` at the start-ish of a statement is a comment.
            // This is inherently ambiguous with string literals so we ONLY
            // treat `"` as a comment when it appears as the first non-blank
            // character on a line.  The caller should use this with the Vim
            // syntax profile which has Double strings — the string matcher
            // is tried first in the main loop, so mid-line `"` will be
            // consumed as a string, and a leading `"` with no closer on the
            // same line will fall through to here.
            //
            // Simplified: just skip for now (Vim is rare).  Treated as Hash
            // but with `"` delimiter would be too ambiguous.  We won't
            // special-case it — just leave as-is and accept minor FP for
            // Vim files.
            None
        }
        Percent => {
            if bytes[i] == b'%' {
                Some(consume_to_eol(bytes, i))
            } else {
                None
            }
        }
    }
}

/// Consume from `start` to end-of-line (or end-of-input).
/// Returns the number of bytes consumed (does NOT consume the newline itself).
fn consume_to_eol(bytes: &[u8], start: usize) -> usize {
    let mut j = start;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    j - start
}

/// Consume from position `start` until we see `end_delim` or hit end-of-input.
/// Returns the number of bytes consumed INCLUDING the end delimiter (minus the
/// opening delimiter which the caller already accounted for).
fn consume_block(bytes: &[u8], start: usize, end_delim: &[u8]) -> usize {
    let dlen = end_delim.len();
    let mut j = start;
    while j + dlen <= bytes.len() {
        if &bytes[j..j + dlen] == end_delim {
            return (j + dlen) - start;
        }
        j += 1;
    }
    // Unterminated — consume to end
    bytes.len() - start
}

// ---------------------------------------------------------------------------
// String matchers
// ---------------------------------------------------------------------------

/// Try to match a string literal starting at position `i`.
/// Returns `Some(byte_count_consumed)` or `None`.
fn try_string(bytes: &[u8], i: usize, syntax: &LangSyntax) -> Option<usize> {
    for style in syntax.strings {
        if let Some(n) = match_string(bytes, i, *style) {
            return Some(n);
        }
    }
    None
}

fn match_string(bytes: &[u8], i: usize, style: StringStyle) -> Option<usize> {
    let remaining = bytes.len() - i;
    match style {
        Double => {
            if bytes[i] == b'"' {
                Some(consume_escaped_string(bytes, i + 1, b'"') + 1)
            } else {
                None
            }
        }
        Single => {
            if bytes[i] == b'\'' {
                Some(consume_escaped_string(bytes, i + 1, b'\'') + 1)
            } else {
                None
            }
        }
        Backtick => {
            if bytes[i] == b'`' {
                Some(consume_escaped_string(bytes, i + 1, b'`') + 1)
            } else {
                None
            }
        }
        TripleDouble => {
            if remaining >= 3 && &bytes[i..i + 3] == b"\"\"\"" {
                Some(consume_triple(bytes, i + 3, b"\"\"\"") + 3)
            } else {
                None
            }
        }
        TripleSingle => {
            if remaining >= 3 && &bytes[i..i + 3] == b"'''" {
                Some(consume_triple(bytes, i + 3, b"'''") + 3)
            } else {
                None
            }
        }
        PythonRawDouble => {
            // r" or R"
            if remaining >= 2
                && (bytes[i] == b'r' || bytes[i] == b'R')
                && bytes[i + 1] == b'"'
            {
                // Raw strings don't process backslash escapes, but we still
                // need to find the closing `"`
                Some(consume_raw_simple(bytes, i + 2, b'"') + 2)
            } else {
                None
            }
        }
        PythonRawSingle => {
            if remaining >= 2
                && (bytes[i] == b'r' || bytes[i] == b'R')
                && bytes[i + 1] == b'\''
            {
                Some(consume_raw_simple(bytes, i + 2, b'\'') + 2)
            } else {
                None
            }
        }
        RustRaw => {
            // r"...", r#"..."#, r##"..."##, etc.
            if bytes[i] != b'r' {
                return None;
            }
            // Count hashes
            let mut hashes = 0usize;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'"' {
                return None;
            }
            j += 1; // skip opening "

            // Build the closing delimiter: "###
            let mut close = vec![b'"'];
            for _ in 0..hashes {
                close.push(b'#');
            }

            // Find the closing delimiter
            while j + close.len() <= bytes.len() {
                if &bytes[j..j + close.len()] == close.as_slice() {
                    return Some((j + close.len()) - i);
                }
                j += 1;
            }
            // Unterminated
            Some(bytes.len() - i)
        }
    }
}

/// Consume an escape-aware string body starting just AFTER the opening quote.
/// Returns bytes consumed INCLUDING the closing quote.
fn consume_escaped_string(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut j = start;
    while j < bytes.len() {
        if bytes[j] == b'\\' {
            j += 2; // skip escaped char
            continue;
        }
        if bytes[j] == quote {
            return (j + 1) - start; // include closing quote
        }
        j += 1;
    }
    // Unterminated — consume to end
    bytes.len() - start
}

/// Consume a raw string (no escape processing) until `quote`.
fn consume_raw_simple(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut j = start;
    while j < bytes.len() {
        if bytes[j] == quote {
            return (j + 1) - start;
        }
        j += 1;
    }
    bytes.len() - start
}

/// Consume a triple-quoted string body starting just AFTER the opening `"""`/`'''`.
/// Returns bytes consumed INCLUDING the closing triple quote.
fn consume_triple(bytes: &[u8], start: usize, delim: &[u8]) -> usize {
    let dlen = delim.len();
    let mut j = start;
    while j + dlen <= bytes.len() {
        // Triple-quoted strings still honour `\"` / `\'` escapes
        if bytes[j] == b'\\' {
            j += 2;
            continue;
        }
        if &bytes[j..j + dlen] == delim {
            return (j + dlen) - start;
        }
        j += 1;
    }
    bytes.len() - start
}

// ---------------------------------------------------------------------------
// Helper: blank bytes preserving newlines
// ---------------------------------------------------------------------------

fn blank_preserving_newlines(span: &[u8], out: &mut Vec<u8>) {
    for &b in span {
        if b == b'\n' {
            out.push(b'\n');
        } else {
            out.push(b' ');
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helper --
    fn san(input: &str, ext: &str) -> String {
        let syntax = syntax_for_extension(ext);
        sanitize(input, syntax)
    }

    // ======================================================================
    // C-style comments
    // ======================================================================

    #[test]
    fn c_line_comment_stripped() {
        let input = "int x = 1; // { not a brace\nint y = 2;";
        let result = san(input, "c");
        assert!(!result.contains('{'));
        assert!(result.contains("int x = 1;"));
        assert!(result.contains("int y = 2;"));
    }

    #[test]
    fn c_block_comment_stripped() {
        let input = "int x = /* { [ ( */ 1;";
        let result = san(input, "c");
        assert!(!result.contains('{'));
        assert!(!result.contains('['));
        assert!(!result.contains('('));
    }

    #[test]
    fn c_block_comment_multiline() {
        let input = "a\n/* {\n [ \n */ b";
        let result = san(input, "c");
        // Should preserve 4 lines
        assert_eq!(result.lines().count(), 4);
        assert!(!result.contains('{'));
        assert!(!result.contains('['));
    }

    // ======================================================================
    // Python comments and strings
    // ======================================================================

    #[test]
    fn python_hash_comment() {
        let input = "x = 1  # { not a brace\ny = 2";
        let result = san(input, "py");
        assert!(!result.contains('{'));
    }

    #[test]
    fn python_triple_double_string() {
        let input = r#"x = """{ [ ( not braces }"""  "#;
        let result = san(input, "py");
        assert!(!result.contains('{'));
        assert!(!result.contains('['));
    }

    #[test]
    fn python_triple_single_string() {
        let input = "x = '''{ [ ('''";
        let result = san(input, "py");
        assert!(!result.contains('{'));
    }

    #[test]
    fn python_raw_string() {
        let input = r#"x = r"{ not a brace }""#;
        let result = san(input, "py");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // Rust comments and strings
    // ======================================================================

    #[test]
    fn rust_line_comment() {
        let input = "let x = 1; // { unclosed\nlet y = 2;";
        let result = san(input, "rs");
        assert!(!result.contains('{'));
    }

    #[test]
    fn rust_raw_string() {
        let input = r###"let s = r#"{ not a brace }"#;"###;
        let result = san(input, "rs");
        // The braces inside the raw string should be blanked
        // Only the semicolon brace-relevant chars should remain from outside
        assert!(!result.contains('{'));
    }

    #[test]
    fn rust_double_string_with_escape() {
        let input = r#"let s = "hello \"world\" { notabrace }";"#;
        let result = san(input, "rs");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // JS/TS template literals
    // ======================================================================

    #[test]
    fn js_backtick_template() {
        let input = "let s = `hello { world }`;";
        let result = san(input, "js");
        assert!(!result.contains('{'));
    }

    #[test]
    fn ts_backtick_template() {
        let input = "const x = `{ braces } inside`;";
        let result = san(input, "ts");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // Haskell
    // ======================================================================

    #[test]
    fn haskell_line_comment() {
        let input = "x = 1 -- { not brace\ny = 2";
        let result = san(input, "hs");
        assert!(!result.contains('{'));
    }

    #[test]
    fn haskell_block_comment() {
        let input = "x = {- { [ ( -} 1";
        let result = san(input, "hs");
        assert!(!result.contains('['));
        assert!(!result.contains('('));
    }

    // ======================================================================
    // OCaml
    // ======================================================================

    #[test]
    fn ocaml_block_comment() {
        let input = "let x = (* { [ *) 1";
        let result = san(input, "ml");
        assert!(!result.contains('{'));
        assert!(!result.contains('['));
    }

    // ======================================================================
    // HTML/XML
    // ======================================================================

    #[test]
    fn html_comment() {
        let input = "<div><!-- { [ ( --></div>";
        let result = san(input, "html");
        assert!(!result.contains('{'));
        assert!(!result.contains('['));
    }

    // ======================================================================
    // SQL
    // ======================================================================

    #[test]
    fn sql_line_comment() {
        let input = "SELECT 1; -- { not\nSELECT 2;";
        let result = san(input, "sql");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // Erlang
    // ======================================================================

    #[test]
    fn erlang_percent_comment() {
        let input = "X = 1. % { not\nY = 2.";
        let result = san(input, "erl");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // Clojure
    // ======================================================================

    #[test]
    fn clojure_semicolon_comment() {
        let input = "(def x 1) ; { not\n(def y 2)";
        let result = san(input, "clj");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // JSON — strings only
    // ======================================================================

    #[test]
    fn json_string_stripped() {
        let input = r#"{"key": "value { } [ ]"}"#;
        let result = san(input, "json");
        // The braces inside the string value should be blanked,
        // but the structural { } should remain
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
        // Count structural braces: should be exactly { and }
        let braces: Vec<char> = result.chars().filter(|c| *c == '{' || *c == '}').collect();
        assert_eq!(braces.len(), 2);
    }

    // ======================================================================
    // Escaped quotes inside strings
    // ======================================================================

    #[test]
    fn escaped_double_quote_in_string() {
        // The \" should NOT end the string
        let input = r#""hello \" { still inside""#;
        let result = san(input, "json");
        assert!(!result.contains('{'));
    }

    #[test]
    fn escaped_single_quote_in_string() {
        let input = r"'hello \' { still inside'";
        let result = san(input, "js");
        assert!(!result.contains('{'));
    }

    #[test]
    fn escaped_backtick_in_template() {
        let input = r"`hello \` { still inside`";
        let result = san(input, "js");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // Line preservation
    // ======================================================================

    #[test]
    fn line_count_preserved_after_sanitize() {
        let input = "line1\n\"str { \\n\ncontd\"\nline4";
        let result = san(input, "json");
        assert_eq!(input.lines().count(), result.lines().count());
    }

    // ======================================================================
    // Length preservation
    // ======================================================================

    #[test]
    fn byte_length_preserved() {
        let input = "/* { block } */\n\"string { }\"\n// line { }";
        let result = san(input, "rs");
        assert_eq!(input.len(), result.len());
    }

    // ======================================================================
    // Mixed: real braces survive
    // ======================================================================

    #[test]
    fn real_braces_survive_sanitize() {
        let input = "fn main() { let x = [1]; } // { comment }";
        let result = san(input, "rs");
        // Structural braces survive
        assert!(result.contains("fn main()"));
        assert!(result.contains("{ let x = [1]; }"));
        // Comment braces gone
        let comment_part = &result[result.find("//").unwrap_or(result.len())..];
        assert!(!comment_part.contains('{'));
    }

    // ======================================================================
    // PHP has three comment styles
    // ======================================================================

    #[test]
    fn php_all_comment_styles() {
        let input = "$x = 1; // { a\n$y = 2; /* { b */ $z = 3; # { c\n$w = 4;";
        let result = san(input, "php");
        let brace_count = result.chars().filter(|c| *c == '{').count();
        assert_eq!(brace_count, 0);
    }

    // ======================================================================
    // Kotlin triple-quoted string
    // ======================================================================

    #[test]
    fn kotlin_triple_string() {
        let input = "val s = \"\"\"{ [ ( not braces }\"\"\"\nval x = 1";
        let result = san(input, "kt");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // F# has // and (* *)
    // ======================================================================

    #[test]
    fn fsharp_both_comment_styles() {
        let input = "let x = 1 // { a\nlet y = (* { b *) 2";
        let result = san(input, "fs");
        let brace_count = result.chars().filter(|c| *c == '{').count();
        assert_eq!(brace_count, 0);
    }

    // ======================================================================
    // TOML / YAML
    // ======================================================================

    #[test]
    fn toml_hash_comment_and_string() {
        let input = "key = \"value { }\" # { comment }";
        let result = san(input, "toml");
        let brace_count = result.chars().filter(|c| *c == '{' || *c == '}').count();
        assert_eq!(brace_count, 0);
    }

    #[test]
    fn yaml_hash_comment() {
        let input = "key: value # { not a brace }\nother: 1";
        let result = san(input, "yml");
        assert!(!result.contains('{'));
    }

    // ======================================================================
    // No syntax (unknown extension) — pass through unchanged
    // ======================================================================

    #[test]
    fn unknown_extension_passthrough() {
        let input = "{ [ ( ) ] } // not stripped # not stripped";
        let result = san(input, "xyz");
        assert_eq!(input, result);
    }
}
