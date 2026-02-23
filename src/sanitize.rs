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
    /// `--[[...]]`, `--[=[...]=]`, etc. (Lua block comment)
    LuaBlock,
    /// `"...` to end of line  (Vim)
    VimLineComment,
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
    /// `R"tag(...)tag"`  C++ raw string (optional tag)
    CppRaw,
    /// `@"..."` with doubled `""` escapes (C# verbatim)
    CSharpVerbatim,
    /// `[[ ... ]]`, `[=[ ... ]=]`, etc.  Lua long string / block
    LuaLongString,
    /// `~r/.../`, `~w{...}`, etc.  Elixir sigils (non-quote delimiter)
    ElixirSigil,
    /// `%q{...}`, `%w[...]`, etc.  Ruby percent literals
    RubyPercentLiteral,
    /// `$$...$$`, `$tag$...$tag$`  PostgreSQL dollar-quoting
    SqlDollar,
    /// `<<LABEL ... LABEL`  shell/ruby/php heredoc
    Heredoc,
    /// `@"..."@` / `@'...'@` (PowerShell here-string)
    PowerShellHereString,
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

static CPP_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[CppRaw, Double, Single],
};

static GO_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[Backtick, Double, Single],
};

static CSHARP_STYLE: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[CSharpVerbatim, Double, Single],
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
    strings: &[SqlDollar, Single],
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
    strings: &[TripleDouble, Double, Single, ElixirSigil],
};

static PHP: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment, Hash],
    strings: &[Heredoc, Double, Single],
};

static LUA: LangSyntax = LangSyntax {
    comments: &[LuaBlock, DoubleDash],
    strings: &[LuaLongString, Double, Single],
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
    strings: &[TripleDouble, TripleSingle, Double, Single],
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
    strings: &[PowerShellHereString, Double, Single],
};

static EMACS_LISP: LangSyntax = LangSyntax {
    comments: &[Semicolon],
    strings: &[Double],
};

static VIM: LangSyntax = LangSyntax {
    comments: &[VimLineComment],
    strings: &[],
};

static GRAPHQL: LangSyntax = LangSyntax {
    comments: &[Hash, CBlockComment],
    strings: &[TripleDouble, Double],
};

static RUBY: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[TripleDouble, Double, Single, RubyPercentLiteral, Heredoc],
};

static SHELL: LangSyntax = LangSyntax {
    comments: &[Hash],
    strings: &[Heredoc, Double, Single],
};

static SWIFT: LangSyntax = LangSyntax {
    comments: &[CLineComment, CBlockComment],
    strings: &[TripleDouble, Double, Single],
};

static PLAIN: LangSyntax = LangSyntax {
    comments: &[],
    strings: &[],
};

/// Map a file extension to its syntax configuration.
pub fn syntax_for_extension(ext: &str) -> &'static LangSyntax {
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => &C_STYLE_BACKTICK,
        "c" | "h"
        | "java" | "dart"
        | "proto" | "tf" | "hcl" => &C_STYLE,
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => &CPP_STYLE,
        "go" => &GO_STYLE,
        "cs" => &CSHARP_STYLE,
        "css" | "scss" | "sass" | "less" => &C_STYLE,
        "swift" => &SWIFT,
        "kt" | "kts" => &KOTLIN,
        "scala" => &SCALA,
        "groovy" => &GROOVY,
        "rs" => &RUST,
        "py" => &PYTHON,
        "rb" => &RUBY,
        "sh" | "bash" | "zsh" | "fish" => &SHELL,
        "r" | "pl" | "pm" | "cmake" | "dockerfile" => &HASH_ONLY,
        "php" => &PHP,
        "lua" => &LUA,
        "ex" | "exs" => &ELIXIR,
        "erl" | "hrl" => &ERLANG,
        "hs" => &HASKELL,
        "elm" => &ELM,
        "ml" | "mli" => &OCAML,
        "fs" | "fsi" | "fsx" => &FSHARP,
        "clj" | "cljs" | "cljc" => &LISP,
        "sql" => &SQL_STYLE,
        "graphql" | "gql" => &GRAPHQL,
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

/// `--[[...]]`, `--[=[...]=]`, etc. (Lua block comment)
fn parse_lua_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 3 || !input.starts_with(b"--[") {
        return Err(err(input));
    }
    let long = parse_lua_long_string(&input[2..])?;
    let consumed = 2 + long.1.len();
    Ok((&input[consumed..], &input[..consumed]))
}

/// `{- ... -}` — supports arbitrary nesting.
fn parse_haskell_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 2 || &input[..2] != b"{-" {
        return Err(err(input));
    }
    let mut depth: usize = 1;
    let mut pos = 2; // after opening `{-`
    while pos < input.len() {
        if pos + 1 < input.len() && &input[pos..pos + 2] == b"{-" {
            depth += 1;
            pos += 2;
        } else if pos + 1 < input.len() && &input[pos..pos + 2] == b"-}" {
            depth -= 1;
            pos += 2;
            if depth == 0 {
                return Ok((&input[pos..], &input[..pos]));
            }
        } else {
            pos += 1;
        }
    }
    Ok((&input[input.len()..], input))
}

/// `(* ... *)` — supports arbitrary nesting.
fn parse_ocaml_block_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 2 || &input[..2] != b"(*" {
        return Err(err(input));
    }
    let mut depth: usize = 1;
    let mut pos = 2; // after opening `(*`
    while pos < input.len() {
        if pos + 1 < input.len() && &input[pos..pos + 2] == b"(*" {
            depth += 1;
            pos += 2;
        } else if pos + 1 < input.len() && &input[pos..pos + 2] == b"*)" {
            depth -= 1;
            pos += 2;
            if depth == 0 {
                return Ok((&input[pos..], &input[..pos]));
            }
        } else {
            pos += 1;
        }
    }
    Ok((&input[input.len()..], input))
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
    let (body, _) = tag::<_, _, Err<'_>>(b"`" as &[u8]).parse_complete(input)?;
    let mut i = 0usize;
    let mut interp_depth = 0usize;
    while i < body.len() {
        if body[i] == b'\\' && i + 1 < body.len() {
            i += 2;
            continue;
        }
        if i + 1 < body.len() && body[i] == b'$' && body[i + 1] == b'{' {
            interp_depth += 1;
            i += 2;
            continue;
        }
        if body[i] == b'}' && interp_depth > 0 {
            interp_depth -= 1;
            i += 1;
            continue;
        }
        if body[i] == b'`' && interp_depth == 0 {
            let end = 1 + i + 1;
            return Ok((&input[end..], &input[..end]));
        }
        i += 1;
    }
    Ok((&input[input.len()..], input))
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

/// `R"tag(...)tag"` — C++ raw string literal.
fn parse_cpp_raw_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 3 || input[0] != b'R' || input[1] != b'"' {
        return Err(err(input));
    }
    let mut tag_end = 2usize;
    while tag_end < input.len() && input[tag_end] != b'(' {
        tag_end += 1;
    }
    if tag_end >= input.len() {
        return Err(err(input));
    }
    let dtag = &input[2..tag_end];
    let mut close = Vec::with_capacity(2 + dtag.len());
    close.push(b')');
    close.extend_from_slice(dtag);
    close.push(b'"');
    let mut pos = tag_end + 1;
    while pos + close.len() <= input.len() {
        if &input[pos..pos + close.len()] == close.as_slice() {
            let total = pos + close.len();
            return Ok((&input[total..], &input[..total]));
        }
        pos += 1;
    }
    Ok((&input[input.len()..], input))
}

/// `@"..."` — C# verbatim string with doubled quote escapes.
fn parse_csharp_verbatim_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 2 || input[0] != b'@' || input[1] != b'"' {
        return Err(err(input));
    }
    let mut pos = 2usize;
    while pos < input.len() {
        if input[pos] == b'"' {
            if pos + 1 < input.len() && input[pos + 1] == b'"' {
                pos += 2;
            } else {
                pos += 1;
                return Ok((&input[pos..], &input[..pos]));
            }
        } else {
            pos += 1;
        }
    }
    Ok((&input[input.len()..], input))
}

// ---------------------------------------------------------------------------
// New parsers for gaps
// ---------------------------------------------------------------------------

/// `"` to end of line (Vim script).
fn parse_vim_line_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag::<_, _, Err<'_>>(b"\"" as &[u8]).parse_complete(input)?;
    let (rest, _) = take_while::<_, _, Err<'_>>(|b: u8| b != b'\n').parse_complete(rest)?;
    let len = input.len() - rest.len();
    Ok((rest, &input[..len]))
}

/// `[[...]]`, `[=[...]=]`, `[==[...]==]`, etc. — Lua long strings / block bodies.
fn parse_lua_long_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.is_empty() || input[0] != b'[' { return Err(err(input)); }
    let mut eq_count = 0usize;
    let mut pos = 1;
    while pos < input.len() && input[pos] == b'=' { eq_count += 1; pos += 1; }
    if pos >= input.len() || input[pos] != b'[' { return Err(err(input)); }
    pos += 1; // skip second `[`
    let mut close = Vec::with_capacity(2 + eq_count);
    close.push(b']');
    for _ in 0..eq_count { close.push(b'='); }
    close.push(b']');
    while pos + close.len() <= input.len() {
        if &input[pos..pos + close.len()] == close.as_slice() {
            let total = pos + close.len();
            return Ok((&input[total..], &input[..total]));
        }
        pos += 1;
    }
    Ok((&input[input.len()..], input))
}

/// `~r/.../`, `~w{...}`, `~c(...)`, etc. — Elixir sigils with non-quote delimiters.
/// `~s"..."` and `~S"""..."""` are already handled upstream by `Double`/`TripleDouble`.
fn parse_elixir_sigil(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 3 || input[0] != b'~' || !input[1].is_ascii_alphabetic() {
        return Err(err(input));
    }
    let open = input[2];
    let close: u8 = match open {
        b'{' => b'}',
        b'[' => b']',
        b'(' => b')',
        b'/' | b'|' => open,
        _ => return Err(err(input)),
    };
    let body = &input[3..];
    let mut pos = 0;
    let is_bracket_pair = matches!(open, b'{' | b'[' | b'(');
    let mut depth: usize = 1;
    while pos < body.len() {
        if body[pos] == b'\\' && pos + 1 < body.len() {
            pos += 2;
            continue;
        }
        if is_bracket_pair && body[pos] == open {
            depth += 1;
        } else if body[pos] == close {
            if is_bracket_pair {
                depth -= 1;
                if depth == 0 {
                    return Ok((&input[3 + pos + 1..], &input[..3 + pos + 1]));
                }
            } else {
                return Ok((&input[3 + pos + 1..], &input[..3 + pos + 1]));
            }
        } else {
            pos += 1;
            continue;
        }
        pos += 1;
    }
    Ok((&input[input.len()..], input))
}

/// `%q{...}`, `%Q[...]`, `%w(...)`, `%(...)`, etc. — Ruby percent literals.
fn parse_ruby_percent_literal(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.is_empty() || input[0] != b'%' { return Err(err(input)); }
    let (prefix_end, open) = if input.len() >= 2 {
        let c = input[1];
        if matches!(c, b'q' | b'Q' | b'w' | b'W' | b'i' | b'I' | b'r' | b's' | b'x') {
            if input.len() < 3 { return Err(err(input)); }
            (2usize, input[2])
        } else if c.is_ascii_punctuation() && c != b'_' {
            (1usize, c)
        } else {
            return Err(err(input));
        }
    } else {
        return Err(err(input));
    };
    let close: u8 = match open {
        b'{' => b'}',
        b'[' => b']',
        b'(' => b')',
        b'<' => b'>',
        c if c.is_ascii_punctuation() => c,
        _ => return Err(err(input)),
    };
    let body = &input[prefix_end + 1..];
    let mut pos = 0;
    // For bracket-pair delimiters, track nesting depth so %q{ a { b } c } works correctly.
    let is_bracket_pair = matches!(open, b'{' | b'[' | b'(' | b'<');
    let mut depth: usize = 1;
    while pos < body.len() {
        if body[pos] == b'\\' && pos + 1 < body.len() {
            pos += 2;
            continue;
        }
        if is_bracket_pair && body[pos] == open {
            depth += 1;
        } else if body[pos] == close {
            depth -= 1;
            if depth == 0 {
                return Ok((&input[prefix_end + 1 + pos + 1..], &input[..prefix_end + 1 + pos + 1]));
            }
        }
        pos += 1;
    }
    Ok((&input[input.len()..], input))
}

/// `$$...$$` and `$tag$...$tag$` — PostgreSQL dollar-quoting.
fn parse_sql_dollar_quote(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.is_empty() || input[0] != b'$' { return Err(err(input)); }
    let mut tag_end = 1;
    while tag_end < input.len() && input[tag_end] != b'$' {
        let b = input[tag_end];
        if !b.is_ascii_alphanumeric() && b != b'_' { return Err(err(input)); }
        tag_end += 1;
    }
    if tag_end >= input.len() { return Err(err(input)); }
    let tag = &input[..tag_end + 1]; // e.g. `$$` or `$body$`
    let mut pos = tag_end + 1;
    while pos + tag.len() <= input.len() {
        if &input[pos..pos + tag.len()] == tag {
            let total = pos + tag.len();
            return Ok((&input[total..], &input[..total]));
        }
        pos += 1;
    }
    Ok((&input[input.len()..], input))
}

/// `<<LABEL`, `<<~LABEL`, `<<"LABEL"`, `<<<LABEL` (PHP), etc. — heredoc literals.
fn parse_heredoc(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (arrow_len, after_arrows) = if input.starts_with(b"<<<") {
        (3usize, &input[3..])
    } else if input.starts_with(b"<<") {
        (2usize, &input[2..])
    } else {
        return Err(err(input));
    };
    let (modifier_len, label_start) = if !after_arrows.is_empty()
        && matches!(after_arrows[0], b'-' | b'~')
    {
        (1usize, &after_arrows[1..])
    } else {
        (0usize, after_arrows)
    };
    let (label, label_field_len) =
        if !label_start.is_empty() && matches!(label_start[0], b'"' | b'\'' | b'`') {
            let q = label_start[0];
            let mut i = 1;
            while i < label_start.len() && label_start[i] != q { i += 1; }
            if i >= label_start.len() { return Err(err(input)); }
            (&label_start[1..i], i + 1)
        } else {
            let mut i = 0;
            while i < label_start.len()
                && (label_start[i].is_ascii_alphanumeric() || label_start[i] == b'_')
            { i += 1; }
            if i == 0 { return Err(err(input)); }
            (&label_start[..i], i)
        };
    if label.is_empty() { return Err(err(input)); }
    let header_end = arrow_len + modifier_len + label_field_len;
    let mut pos = header_end;
    while pos < input.len() && input[pos] != b'\n' { pos += 1; }
    if pos >= input.len() { return Err(err(input)); }
    pos += 1; // skip opening line's \n
    while pos < input.len() {
        let line_start = pos;
        while pos < input.len() && input[pos] != b'\n' { pos += 1; }
        let raw_line = &input[line_start..pos];
        // Strip trailing \r for CRLF files before label matching.
        let line = if raw_line.ends_with(b"\r") { &raw_line[..raw_line.len() - 1] } else { raw_line };
        let s = line.iter().position(|&b| !matches!(b, b' ' | b'\t')).unwrap_or(line.len());
        let stripped = &line[s..];
        if stripped.starts_with(label) {
            let rest = &stripped[label.len()..];
            let rt = rest.iter().position(|&b| !matches!(b, b' ' | b'\t')).map(|i| &rest[i..]).unwrap_or(&[]);
            if rt.is_empty() || matches!(rt[0], b';' | b',') {
                let total = if pos < input.len() { pos + 1 } else { pos };
                return Ok((&input[total..], &input[..total]));
            }
        }
        if pos < input.len() { pos += 1; }
    }
    Ok((&input[input.len()..], input))
}

/// `@"..."@` / `@'...'@` (PowerShell here-string).
fn parse_powershell_here_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() < 3 || input[0] != b'@' || !matches!(input[1], b'"' | b'\'') {
        return Err(err(input));
    }
    let quote = input[1];
    if input[2] != b'\n' {
        return Err(err(input));
    }
    let close = [quote, b'@'];
    let mut pos = 3usize;
    while pos < input.len() {
        let line_start = pos;
        while pos < input.len() && input[pos] != b'\n' { pos += 1; }
        let raw_line = &input[line_start..pos];
        let line = if raw_line.ends_with(b"\r") { &raw_line[..raw_line.len() - 1] } else { raw_line };
        if line == close {
            let total = if pos < input.len() { pos + 1 } else { pos };
            return Ok((&input[total..], &input[..total]));
        }
        if pos < input.len() { pos += 1; }
    }
    Ok((&input[input.len()..], input))
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn try_parse_comment<'a>(input: &'a [u8], style: CommentStyle) -> IResult<&'a [u8], &'a [u8]> {
    match style {
        CLineComment    => parse_c_line_comment(input),
        CBlockComment   => parse_c_block_comment(input),
        Hash            => parse_hash_comment(input),
        DoubleDash      => parse_double_dash_comment(input),
        HaskellBlock    => parse_haskell_block_comment(input),
        OcamlBlock      => parse_ocaml_block_comment(input),
        HtmlBlock       => parse_html_comment(input),
        Semicolon       => parse_semicolon_comment(input),
        Percent         => parse_percent_comment(input),
        LuaBlock        => parse_lua_block_comment(input),
        VimLineComment  => parse_vim_line_comment(input),
    }
}

fn try_parse_string<'a>(input: &'a [u8], style: StringStyle) -> IResult<&'a [u8], &'a [u8]> {
    match style {
        Double              => parse_double_string(input),
        Single              => parse_single_string(input),
        Backtick            => parse_backtick_string(input),
        TripleDouble        => parse_triple_double(input),
        TripleSingle        => parse_triple_single(input),
        PythonRawDouble     => parse_python_raw_double(input),
        PythonRawSingle     => parse_python_raw_single(input),
        RustRaw             => parse_rust_raw_string(input),
        CppRaw              => parse_cpp_raw_string(input),
        CSharpVerbatim      => parse_csharp_verbatim_string(input),
        LuaLongString       => parse_lua_long_string(input),
        ElixirSigil         => parse_elixir_sigil(input),
        RubyPercentLiteral  => parse_ruby_percent_literal(input),
        SqlDollar           => parse_sql_dollar_quote(input),
        Heredoc             => parse_heredoc(input),
        PowerShellHereString=> parse_powershell_here_string(input),
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

    // =========================================================
    // GAP TESTS — written RED first
    // =========================================================

    // -- GraphQL: # comment + """ SDL docstring ---------------------------

    #[test]
    fn graphql_hash_comment_stripped() {
        // # is the comment char; currently mapped to SQL_STYLE (DoubleDash) — FAILS
        let r = san("# { not a brace }\ntype Query { id: ID }", "graphql");
        assert_eq!(r.chars().filter(|c| *c == '{').count(), 1);
    }

    #[test]
    fn graphql_triple_docstring_stripped() {
        // SDL uses \"\"\" for field descriptions — currently not stripped — FAILS
        let r = san("\"\"\"{ [ description }\"\"\"\ntype Foo { id: ID }", "graphql");
        assert_eq!(r.chars().filter(|c| *c == '{').count(), 1);
    }

    // -- Vim: " is a line comment, NOT a string opener ----------------------

    #[test]
    fn vim_line_comment_stripped() {
        // " starts a comment in Vim; currently no comment style defined — FAILS
        let r = san("call foo() \" { not a brace\nlet y = 1", "vim");
        assert!(!r.contains('{'));
    }

    #[test]
    fn vim_comment_doesnt_eat_real_braces() {
        // Without fix, " opens a "string" that eats {} on next line — FAILS
        let r = san("\" { comment\nlet x = {}", "vim");
        assert!(r.contains("{}"));
    }

    // -- Swift: \"\"\"...\"\"\" multiline strings -----------------------------------

    #[test]
    fn swift_triple_string_stripped() {
        // Swift supports \"\"\"...\"\"\"; currently only C_STYLE (no triple) — FAILS
        let r = san("let s = \"\"\"\n{ [ ( not braces\n\"\"\"\nlet y = 1", "swift");
        assert!(!r.contains('{'));
    }

    // -- Lua: --[[...]] block comments and [[...]] long strings ---------------

    #[test]
    fn lua_block_comment_basic() {
        // --[[ ... ]] not currently handled (only DoubleDash line) — FAILS
        let r = san("--[[ { [ ( not braces ]]", "lua");
        assert!(!r.contains('{'));
    }

    #[test]
    fn lua_block_comment_levels() {
        let r = san("--[==[ { [ ( not braces ]==]", "lua");
        assert!(!r.contains('{'));
    }

    #[test]
    fn lua_block_comment_multiline_body_stripped() {
        let r = san("--[[\n  { [ ( in comment\n]]\nlocal x = {}", "lua");
        assert!(r.contains("{}"));
        assert_eq!(r.matches('{').count(), 1, "comment brace leaked: {r:?}");
    }

    #[test]
    fn lua_long_string_basic() {
        let r = san("local s = [[ { [ ( not braces ]]", "lua");
        assert!(!r.contains('{'));
    }

    #[test]
    fn lua_long_string_levels() {
        let r = san("local s = [==[ { [ not braces ]==]", "lua");
        assert!(!r.contains('{'));
    }

    // -- Nested Haskell {- {- -} -} block comments --------------------------

    #[test]
    fn haskell_nested_block_comment() {
        // take_until "-}" terminates at first -} leaving outer comment open — FAILS
        let r = san("x = {- outer {- inner { } -} still outer {} -} 1", "hs");
        assert!(!r.contains('{'));
    }

    // -- Nested OCaml (* (* *) *) block comments ----------------------------

    #[test]
    fn ocaml_nested_block_comment() {
        let r = san("let x = (* outer (* inner { } *) still outer {} *) 1", "ml");
        assert!(!r.contains('{'));
    }

    // -- PowerShell @"..."@ and @'...'@ here-strings -------------------------

    #[test]
    fn powershell_here_string_double() {
        // @"...\n"@ not currently parsed — FAILS
        let r = san("$x = @\"\n{ [ ( not braces\n\"@\n$y = 1", "ps1");
        assert!(!r.contains('{'));
    }

    #[test]
    fn powershell_here_string_single() {
        let r = san("$x = @'\n{ [ ( not braces\n'@\n$y = 1", "ps1");
        assert!(!r.contains('{'));
    }

    #[test]
    fn powershell_here_string_real_brace_survives_after_block() {
        let r = san("$x = @\"\n{ [ ( in here-string\n\"@\n$y = { ok = 1 }", "ps1");
        assert!(r.contains("{ ok = 1 }"));
    }

    // -- Ruby % literals ----------------------------------------------------

    #[test]
    fn ruby_percent_q_curly() {
        // %q{...} — not parsed — FAILS
        let r = san("s = %q{ { [ ( not braces } }", "rb");
        assert!(!r.contains('['));
    }

    #[test]
    fn ruby_percent_q_square() {
        let r = san("s = %q[ { [ ( not braces ] ]", "rb");
        assert!(!r.contains('{'));
    }

    #[test]
    fn ruby_percent_w_array() {
        let r = san("a = %w[ { [ ( not braces ]", "rb");
        assert!(!r.contains('{'));
    }

    // -- Elixir sigils -------------------------------------------------------

    #[test]
    fn elixir_sigil_s_double() {
        // ~s"..." — not parsed — FAILS
        let r = san("x = ~s\"{ [ ( not braces }\"", "ex");
        assert!(!r.contains('{'));
    }

    #[test]
    fn elixir_sigil_s_triple() {
        let r = san("x = ~S\"\"\"\n{ [ ( not braces\n\"\"\"", "ex");
        assert!(!r.contains('{'));
    }

    #[test]
    fn elixir_sigil_r_slash() {
        let r = san("x = ~r/{ [ ( not braces }/", "ex");
        assert!(!r.contains('{'));
    }

    #[test]
    fn elixir_sigil_w_curly() {
        let r = san("x = ~w{ word1 { word2 }", "ex");
        assert!(!r.contains('{'));
    }

    #[test]
    fn elixir_sigil_r_curly_nested_delims() {
        let r = san("x = ~r{a{b}c}\\nreal = {}", "ex");
        assert!(r.contains("real = {}"));
        assert_eq!(r.matches('{').count(), 1, "nested sigil leaked: {r:?}");
        assert_eq!(r.matches('}').count(), 1, "nested sigil left trailing close brace: {r:?}");
    }

    #[test]
    fn go_raw_backtick_string_is_stripped() {
        let r = san("q := `SELECT * FROM t WHERE x = {1}`\\nreal := map[string]int{}", "go");
        assert!(r.contains("map[string]int{}"));
        assert_eq!(r.matches('{').count(), 1, "go raw string leaked: {r:?}");
    }

    #[test]
    fn toml_multiline_string_is_stripped() {
        let r = san("val = \"\"\"\\n{ [ ( not braces\\n\"\"\"\\nreal = {}", "toml");
        assert!(r.contains("real = {}"));
        assert_eq!(r.matches('{').count(), 1, "toml multiline leaked: {r:?}");
    }

    #[test]
    fn csharp_verbatim_string_is_stripped() {
        let r = san("var s = @\"say \"\"hello\"\" to {nobody}\";\\nvar real = new int[] { 1 };", "cs");
        assert!(r.contains("new int[] { 1 }"));
        assert_eq!(r.matches('{').count(), 1, "csharp verbatim leaked: {r:?}");
    }

    #[test]
    fn js_nested_template_literal_is_stripped() {
        let r = san("const s = `outer ${`inner ${1}`}`;\\nconst real = {};", "js");
        assert!(r.contains("const real = {};"));
        assert_eq!(r.matches('{').count(), 1, "nested template leaked: {r:?}");
    }

    #[test]
    fn cpp_raw_string_is_stripped() {
        let r = san("auto s = R\"tag({ [ ( not braces ) ] })tag\";\\nstd::vector<int> v{};", "cpp");
        assert!(r.contains("v{}"));
        assert_eq!(r.matches('{').count(), 1, "cpp raw string leaked: {r:?}");
    }

    #[test]
    fn css_extension_maps_to_c_style_sanitizer() {
        let r = san("/* { [ ( not braces */\\n.rule { color: red; }", "css");
        assert!(r.contains(".rule { color: red; }"));
        assert_eq!(r.matches('{').count(), 1, "css comment leaked: {r:?}");
    }

    // -- SQL $$ dollar-quoting -----------------------------------------------

    #[test]
    fn sql_dollar_quoting_anonymous() {
        // $$ ... $$ — not parsed — FAILS
        let r = san("$$ { [ ( not braces } $$", "sql");
        assert!(!r.contains('{'));
    }

    #[test]
    fn sql_dollar_quoting_tagged() {
        let r = san("$body$ { [ ( not braces $body$", "sql");
        assert!(!r.contains('{'));
    }

    // -- Heredocs -----------------------------------------------------------

    #[test]
    fn ruby_heredoc_squiggly() {
        let r = san("x = <<~HEREDOC\n  { [ ( not braces\nHEREDOC\ny = 1", "rb");
        assert!(!r.contains('{'));
    }

    #[test]
    fn bash_heredoc_basic() {
        let r = san("cat <<EOF\n{ [ ( not braces\nEOF\nexit 0", "sh");
        assert!(!r.contains('{'));
    }

    #[test]
    fn bash_heredoc_quoted() {
        // <<'EOF' is unexpanded (no var substitution) - still needs stripping
        let r = san("cat <<'EOF'\n{ [ ( not braces\nEOF\n", "sh");
        assert!(!r.contains('{'));
    }

    #[test]
    fn php_heredoc() {
        let r = san("$x = <<<EOT\n{ [ ( not braces\nEOT;\n", "php");
        assert!(!r.contains('{'));
    }

    #[test]
    fn ruby_heredoc_crlf() {
        // CRLF line endings must not prevent the terminator from being recognised.
        let r = san("x = <<~HEREDOC\r\n  { [ ( not braces\r\nHEREDOC\r\nreal = {\r\n", "rb");
        assert!(r.contains('{'), "real {{ should survive after CRLF heredoc; got: {r:?}");
    }
}
