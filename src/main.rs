use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

/// Check balance of paired characters in source files.
///
/// Identifies where opening/closing character balance breaks.
/// Supports arbitrary paired characters: {}, (), [], <>, etc.
#[derive(Parser)]
#[command(
    name = "bracebalance",
    after_help = "Examples:\n  \
        bracebalance myfile.py\n  \
        bracebalance src/\n  \
        bracebalance -p \"()\" script.js\n  \
        bracebalance -p \"{}\" \"()\" \"[]\" \"<>\" src/*.c\n  \
        bracebalance --all main.cpp\n  \
        bracebalance --all ./project/"
)]
struct Cli {
    /// Files or directories to check (directories are scanned recursively for supported extensions)
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Character pairs to check, e.g. "{}" "()" "[]" "<>" (default: () {} [])
    #[arg(short, long, value_name = "AB")]
    pairs: Vec<String>,

    /// Check all common pairs: () {} [] <>
    #[arg(long)]
    all: bool,
}

const DEFAULT_PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']')];
const ALL_PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']'), ('<', '>')];

#[rustfmt::skip]
const SUPPORTED_EXTENSIONS: &[&str] = &[
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
    // Infra / build
    "tf", "hcl", "cmake", "dockerfile",
    // Misc
    "vim", "el",
];

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
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

fn collect_from_dir(dir: &Path, files: &mut Vec<PathBuf>) {
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

fn parse_pairs(pair_strings: &[String]) -> Vec<(char, char)> {
    let mut pairs = Vec::new();
    for s in pair_strings {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 2 {
            eprintln!("[ERROR] Pair must be exactly 2 characters, got: '{s}'");
            process::exit(1);
        }
        pairs.push((chars[0], chars[1]));
    }
    pairs
}

struct StackEntry {
    ch: char,
    line_num: usize,
    line_text: String,
}

struct ErrorEntry {
    line_num: usize,
    line_text: String,
    message: String,
}

fn check_balance(filepath: &PathBuf, pairs: &[(char, char)]) -> bool {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Could not read {}: {e}", filepath.display());
            return false;
        }
    };

    let open_chars: HashMap<char, char> = pairs.iter().map(|&(o, c)| (o, c)).collect();
    let close_to_open: HashMap<char, char> = pairs.iter().map(|&(o, c)| (c, o)).collect();

    let mut open_stack: Vec<StackEntry> = Vec::new();
    let mut errors: Vec<ErrorEntry> = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let line_text = line.trim_end().to_string();

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
                        errors.push(ErrorEntry {
                            line_num,
                            line_text: line_text.clone(),
                            message: format!(
                                "'{}' at line {} does not match '{}' opened at line {}",
                                ch, line_num, top.ch, top.line_num
                            ),
                        });
                    }
                } else {
                    errors.push(ErrorEntry {
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

    // Report
    let pair_labels: String = pairs
        .iter()
        .map(|(o, c)| format!("{o}{c}"))
        .collect::<Vec<_>>()
        .join(" ");

    let separator = "=".repeat(80);
    println!("\n{separator}");
    println!("File: {}  |  Checking: {pair_labels}", filepath.display());
    println!("{separator}");

    let mut has_errors = false;

    if !errors.is_empty() {
        has_errors = true;
        println!("\n[ERROR] {} issue(s) found:", errors.len());
        for err in &errors {
            println!("  Line {}: {}", err.line_num, err.message);
            let truncated: String = err.line_text.chars().take(80).collect();
            println!("           {truncated}");
        }
    }

    if !open_stack.is_empty() {
        has_errors = true;
        println!("\n[ERROR] {} UNCLOSED OPENER(S):", open_stack.len());
        println!("\nThese were NEVER closed:");
        for entry in &open_stack {
            let close = open_chars[&entry.ch];
            let truncated: String = entry.line_text.chars().take(80).collect();
            println!(
                "  Line {}: '{}' (needs '{}')  {truncated}",
                entry.line_num, entry.ch, close
            );
        }
        let closers: String = open_stack
            .iter()
            .rev()
            .map(|e| open_chars[&e.ch])
            .collect();
        println!("\n>>> FIX: Add closing char(s): {closers} <<<");
    }

    if !has_errors {
        println!("[OK] BALANCED: All {pair_labels} pairs match!");
        true
    } else {
        println!("\n[FAILED] File has balance errors!");
        false
    }
}

fn main() {
    let cli = Cli::parse();

    let pairs: Vec<(char, char)> = if cli.all {
        ALL_PAIRS.to_vec()
    } else if !cli.pairs.is_empty() {
        parse_pairs(&cli.pairs)
    } else {
        DEFAULT_PAIRS.to_vec()
    };

    let all_files = collect_files(&cli.files);

    let mut failed_files: Vec<PathBuf> = Vec::new();
    let mut checked = 0;

    for filepath in &all_files {
        if !filepath.exists() {
            eprintln!("[WARNING] File not found: {}", filepath.display());
            continue;
        }
        checked += 1;
        if !check_balance(filepath, &pairs) {
            failed_files.push(filepath.clone());
        }
    }

    // Summary
    if all_files.len() > 1 {
        let separator = "=".repeat(80);
        println!("\n{separator}");
        println!("SUMMARY");
        println!("{separator}");
        println!("Total files checked: {checked}");
        println!("Failed files: {}", failed_files.len());
        if !failed_files.is_empty() {
            println!("\n[FAILED] Files with balance errors:");
            for f in &failed_files {
                println!("  - {}", f.display());
            }
        } else {
            println!("\n[OK] All files balanced!");
        }
    }

    if !failed_files.is_empty() {
        process::exit(1);
    }
}
