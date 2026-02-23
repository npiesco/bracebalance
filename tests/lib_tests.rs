/// Unit tests for the bracebalance library.
///
/// These tests exercise the core logic directly without spawning a subprocess.
use bracebalance::{
    check_balance_str, collect_files, is_supported_extension, parse_pairs, resolve_pairs,
    DEFAULT_PAIRS, ALL_PAIRS,
};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// check_balance_str
// ---------------------------------------------------------------------------

#[test]
fn balanced_simple() {
    let r = check_balance_str("fn foo() { let x = [1, 2, 3]; }", DEFAULT_PAIRS);
    assert!(r.is_balanced);
    assert!(r.mismatches.is_empty());
    assert!(r.unclosed.is_empty());
    assert!(r.fix_suggestion.is_none());
}

#[test]
fn unclosed_opener_is_detected() {
    let r = check_balance_str("fn foo() {", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 1);
    assert_eq!(r.unclosed[0].ch, '{');
    assert!(r.fix_suggestion.is_some());
    assert!(r.fix_suggestion.as_deref().unwrap().contains('}'));
}

#[test]
fn extra_closer_is_detected() {
    let r = check_balance_str("let x = 1; }", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(!r.mismatches.is_empty());
}

#[test]
fn mismatch_detected() {
    let r = check_balance_str("fn foo( }", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
}

#[test]
fn mismatch_does_not_cascade_after_recovery() {
    let r = check_balance_str("[}", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.mismatches.len(), 1, "expected one root mismatch, got: {:?}", r.mismatches);
    assert!(r.unclosed.is_empty(), "stale opener should be recovered, got: {:?}", r.unclosed);
}

#[test]
fn empty_string_is_balanced() {
    let r = check_balance_str("", DEFAULT_PAIRS);
    assert!(r.is_balanced);
}

#[test]
fn multiline_balanced() {
    let src = "fn main() {\n    let v = vec![1, 2, 3];\n}\n";
    let r = check_balance_str(src, DEFAULT_PAIRS);
    assert!(r.is_balanced);
}

#[test]
fn fix_suggestion_correct_order() {
    // Three unclosed openers: ( { [
    let r = check_balance_str("([{", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 3);
    let fix = r.fix_suggestion.unwrap();
    assert_eq!(fix, "}])"); // reverse order
}

#[test]
fn all_pairs_includes_angle() {
    // In ALL_PAIRS mode, < > are also tracked
    let r = check_balance_str("Vec<String", ALL_PAIRS);
    assert!(!r.is_balanced);
    assert!(!r.unclosed.is_empty());
    assert_eq!(r.unclosed[0].ch, '<');
}

#[test]
fn default_pairs_ignores_angle() {
    // With DEFAULT_PAIRS, < and > are not tracked
    let r = check_balance_str("Vec<String", DEFAULT_PAIRS);
    assert!(r.is_balanced);
}

#[test]
fn pair_labels_format() {
    let r = check_balance_str("ok", DEFAULT_PAIRS);
    assert!(r.pair_labels.contains("()"));
    assert!(r.pair_labels.contains("{}"));
    assert!(r.pair_labels.contains("[]"));
}

// ---------------------------------------------------------------------------
// parse_pairs / resolve_pairs
// ---------------------------------------------------------------------------

#[test]
fn parse_pairs_ok() {
    let input: Vec<String> = vec!["()".into(), "{}".into()];
    let result = parse_pairs(&input).unwrap();
    assert_eq!(result, vec![('(', ')'), ('{', '}')]);
}

#[test]
fn parse_pairs_bad_length_errors() {
    let input: Vec<String> = vec!["abc".into()];
    assert!(parse_pairs(&input).is_err());
}

#[test]
fn resolve_pairs_all_flag() {
    let result = resolve_pairs(&[], true).unwrap();
    assert_eq!(result, ALL_PAIRS);
}

#[test]
fn resolve_pairs_custom() {
    let input: Vec<String> = vec!["<>".into()];
    let result = resolve_pairs(&input, false).unwrap();
    assert_eq!(result, vec![('<', '>')]);
}

#[test]
fn resolve_pairs_default_when_empty() {
    let result = resolve_pairs(&[], false).unwrap();
    assert_eq!(result, DEFAULT_PAIRS);
}

// ---------------------------------------------------------------------------
// is_supported_extension / collect_files
// ---------------------------------------------------------------------------

#[test]
fn supported_extensions_recognized() {
    for ext in &["rs", "ts", "tsx", "js", "jsx", "py", "json", "go", "java", "css", "scss", "sass", "less"] {
        let name = format!("file.{ext}");
        let path = Path::new(&name);
        assert!(is_supported_extension(path), "Expected {ext} to be supported");
    }
}

#[test]
fn unsupported_extension_rejected() {
    let path = Path::new("file.xyz123notreal");
    assert!(!is_supported_extension(path));
}

#[test]
fn extension_case_insensitive() {
    // .RS in uppercase should still be recognized
    let path = Path::new("file.RS");
    assert!(is_supported_extension(path));
}

#[test]
fn collect_files_passes_through_single_file() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let single = manifest.join("test_artifacts/valid_nested_1.rs");
    let result = collect_files(&[single.clone()]);
    assert_eq!(result, vec![single]);
}

#[test]
fn collect_files_expands_directory() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("test_artifacts");
    let result = collect_files(&[dir]);
    // Should have found all the artifact files
    assert!(!result.is_empty());
    // All returned paths must be files with supported extensions
    for f in &result {
        assert!(f.is_file());
        assert!(is_supported_extension(f));
    }
}
