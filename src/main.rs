use bracebalance::{
    check_and_print, collect_files, format_summary, resolve_pairs,
};
use clap::Parser;
use std::path::PathBuf;
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

fn main() {
    let cli = Cli::parse();

    let pairs = match resolve_pairs(&cli.pairs, cli.all) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] {e}");
            process::exit(1);
        }
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
        if !check_and_print(filepath, &pairs) {
            failed_files.push(filepath.clone());
        }
    }

    if all_files.len() > 1 {
        print!("{}", format_summary(checked, &failed_files));
    }

    if !failed_files.is_empty() {
        process::exit(1);
    }
}
