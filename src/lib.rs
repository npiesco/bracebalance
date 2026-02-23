pub mod sanitize;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};

pub use sanitize::{sanitize, syntax_for_extension, syntax_for_path};

// ---------------------------------------------------------------------------
// Pair constants
// ---------------------------------------------------------------------------

pub const DEFAULT_PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']')];
pub const ALL_PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']'), ('<', '>')];

// ---------------------------------------------------------------------------
// Supported extensions for directory scanning
// ---------------------------------------------------------------------------

#[rustfmt::skip]
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // TypeScript / JavaScript
    "ts", "tsx", "js", "jsx", "mjs", "cjs",
    // Systems
    "rs", "c", "cpp", "cc", "cxx", "h", "hpp", "hxx",
    // JVM
    "java", "kt", "kts", "scala", "groovy", "clj", "cljs", "cljc",
    // Scripting
    "py", "rb", "php", "lua", "r", "pl", "pm",
    // Go / Swift / Dart
    "go", "swift", "dart",
    // .NET
    "cs", "fs", "fsi", "fsx",
    // Functional
    "hs", "ml", "mli", "ex", "exs", "erl", "hrl", "elm",
    // Frontend frameworks
    "vue", "svelte",
    // Shell
    "sh", "bash", "zsh", "fish", "ps1", "psm1",
    // Data / query
    "sql", "graphql", "gql", "proto",
    // Config / data
    "json", "jsonc", "toml", "yml", "yaml",
    // Markup
    "html", "htm", "xml",
    // Stylesheets
    "css", "scss", "sass", "less",
    // Infra / build
    "tf", "hcl", "cmake", "dockerfile",
    // Misc
    "vim", "el",
];

// ---------------------------------------------------------------------------
// Pair parsing
// ---------------------------------------------------------------------------

/// Parse a slice of 2-character strings like `["()", "{}"]` into char tuples.
/// Returns `Err` with a message on malformed input.
pub fn parse_pairs(pair_strings: &[String]) -> Result<Vec<(char, char)>, String> {
    let mut pairs = Vec::new();
    for s in pair_strings {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 2 {
            return Err(format!(
                "Pair must be exactly 2 characters, got: '{s}'"
            ));
        }
        pairs.push((chars[0], chars[1]));
    }
    Ok(pairs)
}

/// CLI variant — exits with code 1 on bad input.
pub fn parse_pairs_or_exit(pair_strings: &[String]) -> Vec<(char, char)> {
    match parse_pairs(pair_strings) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] {e}");
            process::exit(1);
        }
    }
}

/// Resolve the effective pair list from CLI flags.
pub fn resolve_pairs(
    user_pairs: &[String],
    all: bool,
) -> Result<Vec<(char, char)>, String> {
    if all {
        Ok(ALL_PAIRS.to_vec())
    } else if !user_pairs.is_empty() {
        parse_pairs(user_pairs)
    } else {
        Ok(DEFAULT_PAIRS.to_vec())
    }
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

pub fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Expand a mix of file and directory paths into a flat sorted file list.
pub fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_from_dir(path, &mut files);
        } else {
            files.push(path.clone());
        }
    }
    files
}

/// Recursively collect supported files from a directory.
pub fn collect_from_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        eprintln!("[WARNING] Could not read directory: {}", dir.display());
        return;
    };
    let mut sorted: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    sorted.sort_by_key(|e| e.path());
    for entry in sorted {
        let path = entry.path();
        if path.is_dir() {
            collect_from_dir(&path, files);
        } else if is_supported_extension(&path) {
            files.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// Core balance-check algorithm
// ---------------------------------------------------------------------------

/// A single mismatch between an opener and a closer at a given line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceMismatch {
    pub line_num: usize,
    pub line_text: String,
    pub message: String,
}

/// An opener that was never closed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnclosedOpener {
    pub line_num: usize,
    pub line_text: String,
    pub ch: char,
    pub needs: char,
}

/// The result of running the balance-check algorithm on a piece of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResult {
    pub is_balanced: bool,
    pub pair_labels: String,
    pub mismatches: Vec<BalanceMismatch>,
    pub unclosed: Vec<UnclosedOpener>,
    /// Ordered string of characters that need to be appended to fix the file.
    pub fix_suggestion: Option<String>,
}

/// Pure algorithm — no I/O. Takes text content and pair tuples, returns structured result.
///
/// When `ext` is `Some("rs")` etc., string literals and comments are
/// stripped before checking so that braces inside them don't cause
/// false positives.
pub fn check_balance_str(content: &str, pairs: &[(char, char)]) -> BalanceResult {
    check_balance_str_ext(content, pairs, None)
}

/// Like [`check_balance_str`] but accepts an optional file extension to
/// enable language-aware sanitization of strings and comments.
pub fn check_balance_str_ext(
    content: &str,
    pairs: &[(char, char)],
    ext: Option<&str>,
) -> BalanceResult {
    use std::collections::HashMap;

    // Sanitize: strip string literals and comments so only structural
    // paired characters are checked.
    let sanitized: String;
    let effective = if let Some(ext) = ext {
        let syntax = syntax_for_extension(ext);
        sanitized = sanitize(content, syntax);
        &sanitized
    } else {
        content
    };

    let pair_labels = pairs
        .iter()
        .map(|(o, c)| format!("{o}{c}"))
        .collect::<Vec<_>>()
        .join(" ");

    let open_chars: HashMap<char, char> = pairs.iter().map(|&(o, c)| (o, c)).collect();
    let close_to_open: HashMap<char, char> = pairs.iter().map(|&(o, c)| (c, o)).collect();

    struct StackEntry {
        ch: char,
        line_num: usize,
        line_text: String,
    }

    let mut open_stack: Vec<StackEntry> = Vec::new();
    let mut mismatches: Vec<BalanceMismatch> = Vec::new();

    // Iterate over sanitized text for brace logic, but use original
    // content for line_text in error messages.
    let original_lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in effective.lines().enumerate() {
        let line_num = line_idx + 1;
        // Use original line text for display, sanitized for checking
        let line_text = original_lines
            .get(line_idx)
            .unwrap_or(&"")
            .trim_end()
            .to_string();

        for ch in line.chars() {
            if open_chars.contains_key(&ch) {
                open_stack.push(StackEntry {
                    ch,
                    line_num,
                    line_text: line_text.clone(),
                });
            } else if let Some(&expected_open) = close_to_open.get(&ch) {
                if let Some(top) = open_stack.last() {
                    if top.ch == expected_open {
                        open_stack.pop();
                    } else if open_chars.contains_key(&top.ch) {
                        mismatches.push(BalanceMismatch {
                            line_num,
                            line_text: line_text.clone(),
                            message: format!(
                                "'{}' at line {} does not match '{}' opened at line {}",
                                ch, line_num, top.ch, top.line_num
                            ),
                        });
                        // Recovery strategy: consume the stale opener so one root mismatch
                        // doesn't cascade into many secondary mismatches.
                        open_stack.pop();
                    }
                } else {
                    mismatches.push(BalanceMismatch {
                        line_num,
                        line_text: line_text.clone(),
                        message: format!(
                            "Extra '{}' with no matching '{}'",
                            ch, expected_open
                        ),
                    });
                }
            }
        }
    }

    let unclosed: Vec<UnclosedOpener> = open_stack
        .iter()
        .map(|e| UnclosedOpener {
            line_num: e.line_num,
            line_text: e.line_text.clone(),
            ch: e.ch,
            needs: open_chars[&e.ch],
        })
        .collect();

    let fix_suggestion = if unclosed.is_empty() {
        None
    } else {
        let closers: String = open_stack.iter().rev().map(|e| open_chars[&e.ch]).collect();
        Some(closers)
    };

    let is_balanced = mismatches.is_empty() && unclosed.is_empty();

    BalanceResult {
        is_balanced,
        pair_labels,
        mismatches,
        unclosed,
        fix_suggestion,
    }
}

// ---------------------------------------------------------------------------
// Report formatting  (CLI display)
// ---------------------------------------------------------------------------

/// Format a balance result as CLI-style output. Returns the formatted string.
pub fn format_report(label: &str, result: &BalanceResult) -> String {
    let sep = "=".repeat(80);
    let mut out = String::new();

    out.push_str(&format!("\n{sep}\n"));
    out.push_str(&format!(
        "File: {}  |  Checking: {}\n",
        label, result.pair_labels
    ));
    out.push_str(&format!("{sep}\n"));

    if !result.mismatches.is_empty() {
        out.push_str(&format!(
            "\n[ERROR] {} issue(s) found:\n",
            result.mismatches.len()
        ));
        for m in &result.mismatches {
            out.push_str(&format!("  Line {}: {}\n", m.line_num, m.message));
            let truncated: String = m.line_text.chars().take(80).collect();
            out.push_str(&format!("           {truncated}\n"));
        }
    }

    if !result.unclosed.is_empty() {
        out.push_str(&format!(
            "\n[ERROR] {} UNCLOSED OPENER(S):\n",
            result.unclosed.len()
        ));
        out.push_str("\nThese were NEVER closed:\n");
        for u in &result.unclosed {
            let truncated: String = u.line_text.chars().take(80).collect();
            out.push_str(&format!(
                "  Line {}: '{}' (needs '{}')  {truncated}\n",
                u.line_num, u.ch, u.needs
            ));
        }
        if let Some(fix) = &result.fix_suggestion {
            out.push_str(&format!("\n>>> FIX: Add closing char(s): {fix} <<<\n"));
        }
    }

    if result.is_balanced {
        out.push_str(&format!(
            "[OK] BALANCED: All {} pairs match!\n",
            result.pair_labels
        ));
    } else {
        out.push_str("\n[FAILED] File has balance errors!\n");
    }

    out
}

/// Format a multi-file summary.
pub fn format_summary(checked: usize, failed_files: &[PathBuf]) -> String {
    let sep = "=".repeat(80);
    let mut out = String::new();
    out.push_str(&format!("\n{sep}\nSUMMARY\n{sep}\n"));
    out.push_str(&format!("Total files checked: {checked}\n"));
    out.push_str(&format!("Failed files: {}\n", failed_files.len()));
    if !failed_files.is_empty() {
        out.push_str("\n[FAILED] Files with balance errors:\n");
        for f in failed_files {
            out.push_str(&format!("  - {}\n", f.display()));
        }
    } else {
        out.push_str("\n[OK] All files balanced!\n");
    }
    out
}

// ---------------------------------------------------------------------------
// File-level helper (reads + checks + formats — used by CLI and MCP)
// ---------------------------------------------------------------------------

/// Read a file, run the balance check, return (result, label_for_display).
/// Returns `Err(msg)` if the file cannot be read.
pub fn check_balance_file(
    filepath: &PathBuf,
    pairs: &[(char, char)],
) -> Result<BalanceResult, String> {
    let content = fs::read_to_string(filepath)
        .map_err(|e| format!("Could not read {}: {e}", filepath.display()))?;
    let ext = filepath
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    Ok(check_balance_str_ext(&content, pairs, ext.as_deref()))
}

/// Convenience: check file, print result, return `is_balanced`.
/// Mirrors the old `check_balance()` in main.rs for the CLI.
pub fn check_and_print(filepath: &PathBuf, pairs: &[(char, char)]) -> bool {
    match check_balance_file(filepath, pairs) {
        Ok(result) => {
            print!("{}", format_report(&filepath.display().to_string(), &result));
            result.is_balanced
        }
        Err(e) => {
            eprintln!("[ERROR] {e}");
            false
        }
    }
}
