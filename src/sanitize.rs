//! Source-aware sanitizer built on **nom 8** combinators.
//!
//! Strips string literals and comments so the brace-balance checker only sees
//! structural paired characters.  The sanitizer is a single-pass, left-to-right,
//! deterministic parser.  Every byte inside a recognised string or comment is
//! replaced with a space (`b' '`), except newlines which are preserved so that
//! line numbers stay correct.  The output has the same byte-length as the input.
//!
//! ## Language support
//!
//! A [`LangSyntax`] table selects which comment and string styles apply.
//! [`syntax_for_extension`] maps file extensions to the right table for every
//! language listed in `SUPPORTED_EXTENSIONS`.

use std::path::Path;

use nom::{
    IResult, Parser,
    bytes::{tag, take_while, take_until},
};

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
    /// `; ...` to end of line  (Lisp / Clojure)
    Semicolon,
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
    /// `` `...` `` with `` \` `` escape  (JS / TS template literals)
    Backtick,
    /// `"""..."""`  Python triple-double
    TripleDouble,
    /// `'''...'''`  Python triple-single
    TripleSingle,
    /// `r"..."`  Python raw string (double)
    PythonRawDouble,
    /// `r'...'`  Python raw string (single)
    PythonRawSingle,
    /// `r#"..."#`  Rust raw string (variable hash count)
    RustRaw,
}

/// Combined syntax config for one language family.
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

static JSON_STYLE: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[Double],
};

static JSONC_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[Double],
};

static TOML_STYLE: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

static YAML_STYLE: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

static GROOVY: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

static SCALA: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

static KOTLIN: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

static POWERSHELL: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Double, Single],
};

static EMACS_LISP: LangSyntax = LangSyntax {
    comments: &[Semicolon],
    strings: &[Double],
};

static VIM: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[Double, Single],
};

static PLAIN: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[],
};

/// Map a file extension to its syntax configuration.
pub fn syntax_for_extension(ext: &str) -> &'static LangSyntax {
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => &C_STYLE_BACKTICK,
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx"
        | "java" | "go" | "swift" | "dart" | "cs"
        | "proto" | "tf" | "hcl" => &C_STYLE,
        "kt" | "kts" => &KOTLIN,
        "scala" => &SCALA,
        "groovy" => &GROOVY,
        "rs" => &RUST,
        "py" => &PYTHON,
        "rb" | "sh" | "bash" | "zsh" | "fish" | "r" | "pl" | "pm"
        | "cmake" | "dockerfile" => &HASH_ONLY,
        "php" => &PHP,
        "lua" => &LUA,
        "ex" | "exs" => &ELIXIR,
        "erl" | "hrl" => &ERLANG,
        "hs" => &HASKELL,
        "elm" => &ELM,
        "ml" | "mli" => &OCAML,
        "fs" | "fsi" | "fsx" => &FSHARP,
        "clj" | "cljs" | "cljc" => &LISP,
        "sql" | "graphql" | "gql" => &SQL_STYLE,
        "html" | "htm" | "xml" => &HTML,
        "json" => &JSON_STYLE,
        "jsonc" => &JSONC_STYLE,
        "toml" => &TOML_STYLE,
        "yml" | "yaml" => &YAML_STYLE,
        "vim" => &VIM,
        "el" => &EMACS_LISP,
        "ps1" | "psm1" => &POWERSHELL,
        _ => &PLAIN,
    }
}

/// Get syntax from a file path's extension.
pub fn syntax_for_path(path: &Path) -> &'static LangSyntax {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    syntax_for_extension(&ext)
}

// ===========================================================================
// nom 8 parsers
//
// In nom 8, combinators like `tag`, `take_while`, `take_until` return
// `impl Parser` values.  You call `.parse_complete(input)` on them to obtain
// `IResult`.  Every delimiter match and whitespace skip below uses nom
// combinators; escape-aware body scanning is a small state machine that
// feeds results back through `IResult`.
// ===========================================================================

type Err<'a> = nom::error::Error<&'a [u8]>;

fn err(input: &[u8]) -> nom::Err<Err<'_>> {
    nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
}

// ---------------------------------------------------------------------------
// Comment parsers
// ---------------------------------------------------------------------------

/// `// ...` through end-of-line (newline NOT consumed).
fn parse_c_line_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"//" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `/* ... */` (may span multiple lines).
fn parse_c_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"/*" as &[u8]).parse_complete(input)?;
    match take_until::<_, _, Err<'_>>(b"*/" as &[u8]).parse_complete(rest) {
        Ok((after, _)) => {
            let (after, _) = tag::<_, _, Err<'_>>(b"*/" as &[u8]).parse_complete(after)?;
            let len = input.len() - after.len();
            Ok((after, &input[..len]))
        }
        Err(_) => Ok((&input[input.len()..], input)),
    }
}

/// `# ...` through end-of-line.
fn parse_hash_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"#" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `-- ...` through end-of-line.
fn parse_double_dash_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"--" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `{- ... -}`
fn parse_haskell_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"{-" as &[u8]).parse_complete(input)?;
    match take_until::<_, _, Err<'_>>(b"-}" as &[u8]).parse_complete(rest) {
        Ok((after, _)) => {
            let (after, _) = tag::<_, _, Err<'_>>(b"-}" as &[u8]).parse_complete(after)?;
            let len = input.len() - after.len();
            Ok((after, &input[..len]))
        }
        Err(_) => Ok((&input[input.len()..], input)),
    }
}

/// `(* ... *)`
fn parse_ocaml_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"(*" as &[u8]).parse_complete(input)?;
    match take_until::<_, _, Err<'_>>(b"*)" as &[u8]).parse_complete(rest) {
        Ok((after, _)) => {
            let (after, _) = tag::<_, _, Err<'_>>(b"*)" as &[u8]).parse_complete(after)?;
            let len = input.len() - after.len();
            Ok((after, &input[..len]))
        }
        Err(_) => Ok((&input[input.len()..], input)),
    }
}

/// `<!-- ... -->`
fn parse_html_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"<!--" as &[u8]).parse_complete(input)?;
    match take_until::<_, _, Err<'_>>(b"-->" as &[u8]).parse_complete(rest) {
        Ok((after, _)) => {
            let (after, _) = tag::<_, _, Err<'_>>(b"-->" as &[u8]).parse_complete(after)?;
            let len = input.len() - after.len();
            Ok((after, &input[..len]))
        }
        Err(_) => Ok((&input[input.len()..], input)),
    }
}

/// `; ...` to end-of-line (Lisp).
fn parse_semicolon_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b";" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `% ...` to end-of-line (Erlang).
fn parse_percent_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"%" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

// ---------------------------------------------------------------------------
// String literal parsers
// ---------------------------------------------------------------------------

/// Consume a string body after the opening quote, handling `\x` escapes.
/// On success returns `(remaining, body_including_close_quote)`.
fn parse_escaped_body<'a>(input: &'a [u8], quote: u8) -> IResult<&'a [u8], &'a [u8]> {
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'\\' && i + 1 < input.len() {
            i += 2; // skip escape pair
        } else if input[i] == quote {
            return Ok((&input[i + 1..], &input[..i + 1]));
        } else {
            i += 1;
        }
    }
    Ok((&input[input.len()..], input))
}

/// Consume a raw string body (no escape processing) until `quote`.
fn parse_raw_body<'a>(input: &'a [u8], quote: u8) -> IResult<&'a [u8], &'a [u8]> {
    let mut i = 0;
    while i < input.len() {
        if input[i] == quote {
            return Ok((&input[i + 1..], &input[..i + 1]));
        }
        i += 1;
    }
    Ok((&input[input.len()..], input))
}

/// Consume a triple-quoted body until the three-byte closing delimiter.
fn parse_triple_body<'a>(input: &'a [u8], delim: &[u8]) -> IResult<&'a [u8], &'a [u8]> {
    let dlen = delim.len();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'\\' && i + 1 < input.len() {
            i += 2;
        } else if i + dlen <= input.len() && &input[i..i + dlen] == delim {
            let end = i + dlen;
            return Ok((&input[end..], &input[..end]));
        } else {
            i += 1;
        }
    }
    Ok((&input[input.len()..], input))
}

/// `"..."` with `\"` escape.
fn parse_double_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"\"" as &[u8]).parse_complete(input)?;
    let (rest, _) = parse_escaped_body(rest, b'"')?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `'...'` with `\'` escape.
fn parse_single_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"'" as &[u8]).parse_complete(input)?;
    let (rest, _) = parse_escaped_body(rest, b'\'')?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `` `...` `` with `` \` `` escape.
fn parse_backtick_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"`" as &[u8]).parse_complete(input)?;
    let (rest, _) = parse_escaped_body(rest, b'`')?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `"""..."""`
fn parse_triple_double(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"\"\"\"" as &[u8]).parse_complete(input)?;
    let (rest, _) = parse_triple_body(rest, b"\"\"\"")?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `'''...'''`
fn parse_triple_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"'''" as &[u8]).parse_complete(input)?;
    let (rest, _) = parse_triple_body(rest, b"'''")?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `r"..."` / `R"..."` — Python raw double-quoted.
fn parse_python_raw_double(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.is_empty() || (input[0] != b'r' && input[0] != b'R') {
        return Err(err(input));
    }
    let (after_r, _) = tag::<_, _, Err<'_>>(b"\"" as &[u8]).parse_complete(&input[1..])?;
    let (rest, _) = parse_raw_body(after_r, b'"')?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `r'...'` / `R'...'` — Python raw single-quoted.
fn parse_python_raw_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.is_empty() || (input[0] != b'r' && input[0] != b'R') {
        return Err(err(input));
    }
    let (after_r, _) = tag::<_, _, Err<'_>>(b"'" as &[u8]).parse_complete(&input[1..])?;
    let (rest, _) = parse_raw_body(after_r, b'\'')?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `r"..."`, `r#"..."#`, `r##"..."##`, etc. — Rust raw string.
fn parse_rust_raw_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (after_r, _) = tag::<_, _, Err<'_>>(b"r" as &[u8]).parse_complete(input)?;
    // Count `#`s using take_while
    let (after_hashes, hashes_slice) =
        take_while::<_, _, Err<'_>>(|b: u8| b == b'#').parse_complete(after_r)?;
    let num_hashes = hashes_slice.len();
    // Must open with `"`
    let (body_start, _) = tag::<_, _, Err<'_>>(b"\"" as &[u8]).parse_complete(after_hashes)?;

    // Build closing delimiter: `"` + num_hashes × `#`
    let mut close = Vec::with_capacity(1 + num_hashes);
    close.push(b'"');
    close.extend(std::iter::repeat(b'#').take(num_hashes));

    // Scan for closing delimiter
    let mut pos = 0;
    while pos + close.len() <= body_start.len() {
        if &body_start[pos..pos + close.len()] == close.as_slice() {
            let end_in_body = pos + close.len();
            let total = input.len() - (body_start.len() - end_in_body);
            return Ok((&input[total..], &input[..total]));
        }
        pos += 1;
    }
    // unterminated
    Ok((&input[input.len()..], input))
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn try_parse_comment<'a>(input: &'a [u8], style: CommentStyle) -> IResult<&'a [u8], &'a [u8]> {
    match style {
        CLineComment  => parse_c_line_comment(input),
        CBlockComment => parse_c_block_comment(input),
        Hash          => parse_hash_comment(input),
        DoubleDash    => parse_double_dash_comment(input),
        HaskellBlock  => parse_haskell_block_comment(input),
        OcamlBlock    => parse_ocaml_block_comment(input),
        HtmlBlock     => parse_html_comment(input),
        Semicolon     => parse_semicolon_comment(input),
        Percent       => parse_percent_comment(input),
    }
}

fn try_parse_string<'a>(input: &'a [u8], style: StringStyle) -> IResult<&'a [u8], &'a [u8]> {
    match style {
        Double          => parse_double_string(input),
        Single          => parse_single_string(input),
        Backtick        => parse_backtick_string(input),
        TripleDouble    => parse_triple_double(input),
        TripleSingle    => parse_triple_single(input),
        PythonRawDouble => parse_python_raw_double(input),
        PythonRawSingle => parse_python_raw_single(input),
        RustRaw         => parse_rust_raw_string(input),
    }
}

// ===========================================================================
// Top-level sanitizer
// ===========================================================================

/// Replace every byte inside recognised string literals and comments with
/// spaces, preserving newlines.  The output has the same byte-length and
/// line structure as the input.
pub fn sanitize(input: &str, syntax: &LangSyntax) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut pos = 0usize;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        // 1. Try each comment style.
        let mut matched = false;
        for &style in syntax.comments {
            if let Ok((_, span)) = try_parse_comment(remaining, style) {
                blank_preserving_newlines(span, &mut out);
                pos += span.len();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        // 2. Try each string style.
        for &style in syntax.strings {
            if let Ok((_, span)) = try_parse_string(remaining, style) {
                blank_preserving_newlines(span, &mut out);
                pos += span.len();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        // 3. Pass through regular byte.
        out.push(bytes[pos]);
        pos += 1;
    }

    String::from_utf8(out).expect("sanitize produced invalid UTF-8")
}

/// Replace bytes with spaces, keeping `\n` intact.
fn blank_preserving_newlines(span: &[u8], out: &mut Vec<u8>) {
    for &b in span {
        out.push(if b == b'\n' { b'\n' } else { b' ' });
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn san(input: &str, ext: &str) -> String {
        sanitize(input, syntax_for_extension(ext))
    }

    // -- C-style comments ---------------------------------------------------

    #[test]
    fn c_line_comment_stripped() {
        let r = san("int x = 1; // { not a brace\nint y = 2;", "c");
        assert!(!r.contains('{'));
        assert!(r.contains("int x = 1;"));
        assert!(r.contains("int y = 2;"));
    }

    #[test]
    fn c_block_comment_stripped() {
        let r = san("int x = /* { [ ( */ 1;", "c");
        assert!(!r.contains('{'));
        assert!(!r.contains('['));
        assert!(!r.contains('('));
    }

    #[test]
    fn c_block_comment_multiline() {
        let r = san("a\n/* {\n [ \n */ b", "c");
        assert_eq!(r.lines().count(), 4);
        assert!(!r.contains('{'));
        assert!(!r.contains('['));
    }

    // -- Python -------------------------------------------------------------

    #[test]
    fn python_hash_comment() {
        let r = san("x = 1  # { not a brace\ny = 2", "py");
        assert!(!r.contains('{'));
    }

    #[test]
    fn python_triple_double_string() {
        let r = san(r#"x = """{ [ ( not braces }"""  "#, "py");
        assert!(!r.contains('{'));
        assert!(!r.contains('['));
    }

    #[test]
    fn python_triple_single_string() {
        let r = san("x = '''{ [ ('''", "py");
        assert!(!r.contains('{'));
    }

    #[test]
    fn python_raw_string_double() {
        let r = san(r#"x = r"{ not a brace }""#, "py");
        assert!(!r.contains('{'));
    }

    #[test]
    fn python_raw_string_single() {
        let r = san("x = r'{ not a brace }'", "py");
        assert!(!r.contains('{'));
    }

    // -- Rust ---------------------------------------------------------------

    #[test]
    fn rust_line_comment() {
        let r = san("let x = 1; // { unclosed\nlet y = 2;", "rs");
        assert!(!r.contains('{'));
    }

    #[test]
    fn rust_raw_string_one_hash() {
        let r = san(r###"let s = r#"{ not a brace }"#;"###, "rs");
        assert!(!r.contains('{'));
    }

    #[test]
    fn rust_raw_string_no_hash() {
        let r = san(r#"let s = r"{ brace }";"#, "rs");
        assert!(!r.contains('{'));
    }

    #[test]
    fn rust_double_string_with_escape() {
        let r = san(r#"let s = "hello \"world\" { notabrace }";"#, "rs");
        assert!(!r.contains('{'));
    }

    // -- JS/TS backtick templates -------------------------------------------

    #[test]
    fn js_backtick_template() {
        let r = san("let s = `hello { world }`;", "js");
        assert!(!r.contains('{'));
    }

    #[test]
    fn ts_backtick_template() {
        let r = san("const x = `{ braces } inside`;", "ts");
        assert!(!r.contains('{'));
    }

    #[test]
    fn js_escaped_backtick() {
        let r = san(r"`hello \` { still inside`", "js");
        assert!(!r.contains('{'));
    }

    // -- Haskell ------------------------------------------------------------

    #[test]
    fn haskell_line_comment() {
        let r = san("x = 1 -- { not brace\ny = 2", "hs");
        assert!(!r.contains('{'));
    }

    #[test]
    fn haskell_block_comment() {
        let r = san("x = {- { [ ( -} 1", "hs");
        assert!(!r.contains('['));
        assert!(!r.contains('('));
    }

    // -- OCaml --------------------------------------------------------------

    #[test]
    fn ocaml_block_comment() {
        let r = san("let x = (* { [ *) 1", "ml");
        assert!(!r.contains('{'));
        assert!(!r.contains('['));
    }

    // -- HTML/XML -----------------------------------------------------------

    #[test]
    fn html_comment() {
        let r = san("<div><!-- { [ ( --></div>", "html");
        assert!(!r.contains('{'));
        assert!(!r.contains('['));
    }

    // -- SQL ----------------------------------------------------------------

    #[test]
    fn sql_line_comment() {
        let r = san("SELECT 1; -- { not\nSELECT 2;", "sql");
        assert!(!r.contains('{'));
    }

    // -- Erlang -------------------------------------------------------------

    #[test]
    fn erlang_percent_comment() {
        let r = san("X = 1. % { not\nY = 2.", "erl");
        assert!(!r.contains('{'));
    }

    // -- Clojure ------------------------------------------------------------

    #[test]
    fn clojure_semicolon_comment() {
        let r = san("(def x 1) ; { not\n(def y 2)", "clj");
        assert!(!r.contains('{'));
    }

    // -- JSON ---------------------------------------------------------------

    #[test]
    fn json_string_stripped() {
        let r = san(r#"{"key": "value { } [ ]"}"#, "json");
        assert!(r.starts_with('{'));
        assert!(r.ends_with('}'));
        assert_eq!(r.chars().filter(|c| *c == '{' || *c == '}').count(), 2);
    }

    // -- Escaped quotes -----------------------------------------------------

    #[test]
    fn escaped_double_quote_in_string() {
        let r = san(r#""hello \" { still inside""#, "json");
        assert!(!r.contains('{'));
    }

    #[test]
    fn escaped_single_quote_in_string() {
        let r = san(r"'hello \' { still inside'", "js");
        assert!(!r.contains('{'));
    }

    // -- Preservation guarantees --------------------------------------------

    #[test]
    fn line_count_preserved() {
        let input = "line1\n\"str { \\n\ncontd\"\nline4";
        let r = san(input, "json");
        assert_eq!(input.lines().count(), r.lines().count());
    }

    #[test]
    fn byte_length_preserved() {
        let input = "/* { block } */\n\"string { }\"\n// line { }";
        let r = san(input, "rs");
        assert_eq!(input.len(), r.len());
    }

    // -- Mixed: structural braces survive -----------------------------------

    #[test]
    fn real_braces_survive() {
        let r = san("fn main() { let x = [1]; } // { comment }", "rs");
        assert!(r.contains("{ let x = [1]; }"));
        let after_structural = &r[27..];
        assert!(!after_structural.contains('{'));
    }

    // -- PHP (three comment styles) -----------------------------------------

    #[test]
    fn php_all_comment_styles() {
        let r = san("$x = 1; // { a\n$y = 2; /* { b */ $z = 3; # { c\n$w = 4;", "php");
        assert_eq!(r.chars().filter(|c| *c == '{').count(), 0);
    }

    // -- Kotlin triple-quoted string ----------------------------------------

    #[test]
    fn kotlin_triple_string() {
        let r = san("val s = \"\"\"{ [ ( not braces }\"\"\"\nval x = 1", "kt");
        assert!(!r.contains('{'));
    }

    // -- F# (// and (* *)) -------------------------------------------------

    #[test]
    fn fsharp_both_comment_styles() {
        let r = san("let x = 1 // { a\nlet y = (* { b *) 2", "fs");
        assert_eq!(r.chars().filter(|c| *c == '{').count(), 0);
    }

    // -- TOML / YAML --------------------------------------------------------

    #[test]
    fn toml_hash_comment_and_string() {
        let r = san("key = \"value { }\" # { comment }", "toml");
        assert_eq!(r.chars().filter(|c| *c == '{' || *c == '}').count(), 0);
    }

    #[test]
    fn yaml_hash_comment() {
        let r = san("key: value # { not a brace }\nother: 1", "yml");
        assert!(!r.contains('{'));
    }

    // -- Unknown extension passes through -----------------------------------

    #[test]
    fn unknown_extension_passthrough() {
        let input = "{ [ ( ) ] } // not stripped # not stripped";
        assert_eq!(san(input, "xyz"), input);
    }
}
