pub mod sanitize;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};
use serde_json::json;

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
    /// Number of extra closer messages suppressed because unclosed openers were present.
    pub suppressed_extra_closers: usize,
    /// Suppressed extra-closer details (available for expanded diagnostics).
    pub suppressed_extra_closer_details: Vec<BalanceMismatch>,
    /// Ordered string of characters that need to be appended to fix the file.
    pub fix_suggestion: Option<String>,
}

/// Deduplicated diagnostic location with occurrence count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLocation {
    pub line_num: usize,
    pub line_text: String,
    pub occurrences: usize,
}

/// Structured output payload for MCP/automation consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDiagnostics {
    pub label: String,
    pub balanced: bool,
    pub unclosed: usize,
    pub extra_closers: usize,
    pub suppressed_count: usize,
    pub fix_suggestion: Option<String>,
    pub fallback_fix_suggestion: Option<String>,
    pub locations: Vec<DiagnosticLocation>,
    pub primary_locations: Vec<DiagnosticLocation>,
    pub ranked_repair_hints: Vec<String>,
    pub unclosed_details: Vec<UnclosedOpener>,
    pub extra_closer_details: Vec<BalanceMismatch>,
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

    fn opener_line_context(
        original_lines: &[&str],
        line_idx: usize,
        line_text: &str,
        opener: char,
    ) -> String {
        let trimmed = line_text.trim();
        if trimmed == opener.to_string() {
            if let Some(prev_nonblank) = original_lines[..line_idx]
                .iter()
                .rev()
                .find(|line| !line.trim().is_empty())
            {
                return format!("{}  {}", prev_nonblank.trim_end(), line_text);
            }
        }
        line_text.to_string()
    }

    struct StackEntry {
        ch: char,
        line_num: usize,
        line_text: String,
        seq: usize,
    }

    struct UnclosedForFix {
        entry: UnclosedOpener,
        seq: usize,
    }

    let mut open_stack: Vec<StackEntry> = Vec::new();
    let mut mismatches: Vec<BalanceMismatch> = Vec::new();
    let mut extra_closer_mismatches: Vec<BalanceMismatch> = Vec::new();
    // Openers drained from the stack by the skip-forward recovery strategy.
    // They were never explicitly closed, so they appear in the final unclosed list.
    let mut orphaned_unclosed: Vec<UnclosedForFix> = Vec::new();
    let mut push_seq: usize = 0;

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
                push_seq += 1;
                open_stack.push(StackEntry {
                    ch,
                    line_num,
                    line_text: opener_line_context(
                        &original_lines,
                        line_idx,
                        &line_text,
                        ch,
                    ),
                    seq: push_seq,
                });
            } else if let Some(&expected_open) = close_to_open.get(&ch) {
                // Skip-forward recovery: search the entire stack for the matching opener.
                // If found at position `pos`, every entry above `pos` is an "orphaned"
                // opener (never explicitly closed) — drain them into orphaned_unclosed so
                // they appear in the output, then silently pop the match.
                // If NOT found, report this as a genuine extra closer and leave the stack
                // intact so the current top can still match a future closer.
                if let Some(pos) = open_stack.iter().rposition(|e| e.ch == expected_open) {
                    // Drain entries above the match into orphaned_unclosed.
                    let drained: Vec<StackEntry> = open_stack.drain((pos + 1)..).collect();
                    for entry in drained {
                        orphaned_unclosed.push(UnclosedForFix {
                            entry: UnclosedOpener {
                                line_num: entry.line_num,
                                line_text: entry.line_text,
                                ch: entry.ch,
                                needs: open_chars[&entry.ch],
                            },
                            seq: entry.seq,
                        });
                    }
                    // Pop the match (now at the top after the drain).
                    open_stack.pop();
                } else {
                    // No matching opener anywhere — genuinely extra closer.
                    // Do NOT pop: the current top may still close a future match.
                    extra_closer_mismatches.push(BalanceMismatch {
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

    // Merge remaining open_stack with orphaned_unclosed.
    let mut unclosed_for_fix: Vec<UnclosedForFix> = open_stack
        .iter()
        .map(|e| UnclosedForFix {
            entry: UnclosedOpener {
                line_num: e.line_num,
                line_text: e.line_text.clone(),
                ch: e.ch,
                needs: open_chars[&e.ch],
            },
            seq: e.seq,
        })
        .collect();

    // Track counts BEFORE merging — needed for fix_suggestion reliability guard.
    let orphaned_count = orphaned_unclosed.len();
    let extra_closer_count = extra_closer_mismatches.len();

    unclosed_for_fix.extend(orphaned_unclosed);

    // Suppress pure extra-closer noise when we already have unclosed openers.
    // This keeps reports focused on primary root causes.
    let suppressed_extra_closer_details = if unclosed_for_fix.is_empty() {
        Vec::new()
    } else {
        extra_closer_mismatches.clone()
    };

    let suppressed_extra_closers = if unclosed_for_fix.is_empty() {
        mismatches.extend(extra_closer_mismatches);
        0
    } else {
        suppressed_extra_closer_details.len()
    };

    // Public unclosed list: line-ordered for readability.
    let mut unclosed: Vec<UnclosedOpener> = unclosed_for_fix
        .iter()
        .map(|u| u.entry.clone())
        .collect();
    unclosed.sort_by_key(|u| u.line_num);

    // Fix suggestion: ONLY emit an append-fix when it's guaranteed to work.
    // Appending closers is correct when ALL unclosed openers are from the
    // regular stack (no skip-forward orphaning) AND there are no extra closers
    // in the file body.  When orphaned openers or extra closers exist, the
    // interleaving is too complex for a simple append — require manual fix.
    let fix_suggestion = if unclosed.is_empty()
        || orphaned_count > 0
        || extra_closer_count > 0
    {
        None
    } else {
        unclosed_for_fix.sort_by_key(|u| u.seq);
        let closers: String = unclosed_for_fix
            .iter()
            .rev()
            .map(|u| u.entry.needs)
            .collect();
        Some(closers)
    };

    let is_balanced = mismatches.is_empty() && unclosed.is_empty();

    BalanceResult {
        is_balanced,
        pair_labels,
        mismatches,
        unclosed,
        suppressed_extra_closers,
        suppressed_extra_closer_details,
        fix_suggestion,
    }
}

fn is_extra_closer_message(message: &str) -> bool {
    message.starts_with("Extra '")
}

pub fn dedup_locations(result: &BalanceResult) -> Vec<DiagnosticLocation> {
    dedup_locations_with_suppressed(result, false)
}

pub fn dedup_primary_locations(result: &BalanceResult) -> Vec<DiagnosticLocation> {
    use std::collections::BTreeMap;

    let mut by_location: BTreeMap<(usize, String), usize> = BTreeMap::new();

    if !result.unclosed.is_empty() {
        for u in &result.unclosed {
            *by_location
                .entry((u.line_num, u.line_text.clone()))
                .or_insert(0) += 1;
        }
    } else {
        for m in &result.mismatches {
            *by_location
                .entry((m.line_num, m.line_text.clone()))
                .or_insert(0) += 1;
        }
    }

    let mut locations = by_location
        .into_iter()
        .map(|((line_num, line_text), occurrences)| DiagnosticLocation {
            line_num,
            line_text,
            occurrences,
        })
        .collect::<Vec<_>>();

    // Prioritize highest-density and earliest lines, and cap to keep signal focused.
    locations.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then(a.line_num.cmp(&b.line_num))
    });

    const MAX_PRIMARY_LOCATIONS: usize = 5;
    if locations.len() > MAX_PRIMARY_LOCATIONS {
        locations.truncate(MAX_PRIMARY_LOCATIONS);
    }

    locations.sort_by_key(|l| l.line_num);
    locations
}

pub fn fallback_fix_suggestion(result: &BalanceResult) -> Option<String> {
    if result.is_balanced || result.fix_suggestion.is_some() {
        return None;
    }

    let primary = dedup_primary_locations(result);
    let start_line = primary.first().map(|l| l.line_num).unwrap_or(1);

    let mut text = format!(
        "Manual repair: start at line {start_line}, restore local nesting first, then re-run expanded diagnostics."
    );
    if result.suppressed_extra_closers > 0 {
        text.push_str(&format!(
            " Review {} suppressed extra closer(s) to remove/relocate stray closers before final balancing.",
            result.suppressed_extra_closers
        ));
    }
    Some(text)
}

fn dedup_locations_with_suppressed(
    result: &BalanceResult,
    include_suppressed: bool,
) -> Vec<DiagnosticLocation> {
    use std::collections::BTreeMap;

    let mut by_location: BTreeMap<(usize, String), usize> = BTreeMap::new();

    for u in &result.unclosed {
        *by_location
            .entry((u.line_num, u.line_text.clone()))
            .or_insert(0) += 1;
    }
    for m in &result.mismatches {
        *by_location
            .entry((m.line_num, m.line_text.clone()))
            .or_insert(0) += 1;
    }
    if include_suppressed {
        for m in &result.suppressed_extra_closer_details {
            *by_location
                .entry((m.line_num, m.line_text.clone()))
                .or_insert(0) += 1;
        }
    }

    by_location
        .into_iter()
        .map(|((line_num, line_text), occurrences)| DiagnosticLocation {
            line_num,
            line_text,
            occurrences,
        })
        .collect()
}

pub fn ranked_repair_hints(result: &BalanceResult) -> Vec<String> {
    let mut hints: Vec<String> = Vec::new();

    if result.is_balanced {
        hints.push("No action needed: all selected pairs are balanced.".to_string());
        return hints;
    }

    if let Some(fix) = &result.fix_suggestion {
        hints.push(format!(
            "Append the suggested closers at EOF: {fix}"
        ));
    } else if !result.unclosed.is_empty() {
        hints.push(
            "Resolve interleaved/mismatched nesting around the first affected location before appending closers."
                .to_string(),
        );
    }

    if result.suppressed_extra_closers > 0 {
        hints.push(
            "Inspect suppressed extra closers in expanded diagnostics to identify early stray closers."
                .to_string(),
        );
    }

    let locations = dedup_primary_locations(result);
    if let Some(first) = locations.first() {
        hints.push(format!(
            "Start triage at line {} and fix from top-to-bottom to reduce cascade effects.",
            first.line_num
        ));
    }

    hints
}

pub fn build_structured_diagnostics(
    label: &str,
    result: &BalanceResult,
    expanded: bool,
) -> StructuredDiagnostics {
    let mut extra_closer_details = result
        .mismatches
        .iter()
        .filter(|m| is_extra_closer_message(&m.message))
        .cloned()
        .collect::<Vec<_>>();
    extra_closer_details.extend(result.suppressed_extra_closer_details.clone());

    StructuredDiagnostics {
        label: label.to_string(),
        balanced: result.is_balanced,
        unclosed: result.unclosed.len(),
        extra_closers: extra_closer_details.len(),
        suppressed_count: result.suppressed_extra_closers,
        fix_suggestion: result.fix_suggestion.clone(),
        fallback_fix_suggestion: fallback_fix_suggestion(result),
        locations: dedup_locations_with_suppressed(result, expanded),
        primary_locations: dedup_primary_locations(result),
        ranked_repair_hints: ranked_repair_hints(result),
        unclosed_details: if expanded {
            result.unclosed.clone()
        } else {
            Vec::new()
        },
        extra_closer_details: if expanded {
            extra_closer_details
        } else {
            Vec::new()
        },
    }
}

pub fn format_report_expanded(label: &str, result: &BalanceResult) -> String {
    let mut out = format_report(label, result);

    if result.suppressed_extra_closers > 0 {
        out.push_str("\n[TRACE] Suppressed extra closer details:\n");
        for m in &result.suppressed_extra_closer_details {
            let truncated: String = m.line_text.chars().take(120).collect();
            out.push_str(&format!(
                "  Line {}: {}\n           {}\n",
                m.line_num, m.message, truncated
            ));
        }
    }

    let hints = ranked_repair_hints(result);
    if !hints.is_empty() {
        out.push_str("\n[HINTS] Ranked repair hints:\n");
        for (idx, hint) in hints.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", idx + 1, hint));
        }
    }

    out
}

pub fn format_structured_json(label: &str, result: &BalanceResult, expanded: bool) -> String {
    let payload = build_structured_diagnostics(label, result, expanded);
    json!(payload).to_string()
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
            let truncated: String = m.line_text.chars().take(120).collect();
            out.push_str(&format!("           {truncated}\n"));
        }
    }

    if !result.unclosed.is_empty() {
        out.push_str(&format!(
            "\n[ERROR] {} UNCLOSED OPENER(S):\n",
            result.unclosed.len()
        ));
        out.push_str("\nThese were NEVER closed:\n");
        const CONCISE_UNCLOSED_LIMIT: usize = 8;
        let show_all_unclosed = result.unclosed.len() <= CONCISE_UNCLOSED_LIMIT;
        let shown_unclosed = if show_all_unclosed {
            result.unclosed.len()
        } else {
            CONCISE_UNCLOSED_LIMIT
        };

        for u in result.unclosed.iter().take(shown_unclosed) {
            let truncated: String = u.line_text.chars().take(120).collect();
            out.push_str(&format!(
                "  Line {}: '{}' (needs '{}')  {truncated}\n",
                u.line_num, u.ch, u.needs
            ));
        }

        if !show_all_unclosed {
            let omitted = result.unclosed.len() - shown_unclosed;
            out.push_str(&format!(
                "  ... {} more omitted (Showing first {})\n",
                omitted, shown_unclosed
            ));
        }

        if let Some(fix) = &result.fix_suggestion {
            out.push_str(&format!("\n>>> FIX: Add closing char(s): {fix} <<<\n"));
        } else {
            out.push_str("\n[NOTE] Interleaved / mismatched brackets detected — manual fix required.\n");
        }
    }

    let locations = dedup_locations(result);
    if !locations.is_empty() {
        out.push_str("\n[INFO] Affected locations (deduped):\n");
        for loc in locations {
            let truncated: String = loc.line_text.chars().take(120).collect();
            out.push_str(&format!(
                "  Line {} (x{}): {}\n",
                loc.line_num, loc.occurrences, truncated
            ));
        }
    }

    let primary_locations = dedup_primary_locations(result);
    if !primary_locations.is_empty() {
        out.push_str("\n[INFO] Primary locations:\n");
        for loc in primary_locations {
            let truncated: String = loc.line_text.chars().take(120).collect();
            out.push_str(&format!(
                "  Line {} (x{}): {}\n",
                loc.line_num, loc.occurrences, truncated
            ));
        }
    }

    if result.suppressed_extra_closers > 0 {
        out.push_str(&format!(
            "\n[INFO] Suppressed {} extra closer message(s) to focus on unclosed openers.\n",
            result.suppressed_extra_closers
        ));
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
