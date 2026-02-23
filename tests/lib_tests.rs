/// Unit tests for the bracebalance library.
///
/// These tests exercise the core logic directly without spawning a subprocess.
use bracebalance::{
    build_structured_diagnostics,
    check_balance_file, check_balance_str, check_balance_str_ext,
    collect_files, dedup_locations, format_report, format_report_expanded,
    is_supported_extension, parse_pairs, resolve_pairs,
    DEFAULT_PAIRS, ALL_PAIRS,
};
use serde_json::json;
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

// ---------------------------------------------------------------------------
// Issue 1: skip-forward algorithm reduces cascade noise
// Issue 2: fix_suggestion covers orphaned openers from skip-forward
// ---------------------------------------------------------------------------

#[test]
fn mismatch_does_not_cascade_after_recovery() {
    // `[}` — `}` has no matching `{` in the stack.
    // The `[` should NOT be consumed; it remains as an unclosed opener.
    // No cascade: extra-closer is suppressed; 1 unclosed remains.
    let r = check_balance_str("[}", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(r.mismatches.is_empty(), "expected no user-facing mismatch noise, got: {:?}", r.mismatches);
    assert_eq!(r.unclosed.len(), 1, "[ should remain unclosed, got: {:?}", r.unclosed);
    assert_eq!(r.unclosed[0].ch, '[');
}

#[test]
fn skip_forward_reduces_cascade() {
    // `[{]}` — old algorithm: 2 mismatches, 0 unclosed.
    // skip-forward: `]` looks into stack, finds `[` at depth 0, silently drains `{`
    // into orphaned-unclosed, matches `[`, then `}` finds empty stack → 1 extra-closer.
    // Result: 0 user-facing mismatches (extra closer is suppressed), 1 unclosed (`{`).
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.mismatches.len(), 0, "extra-closer noise should be suppressed, got: {:?}", r.mismatches);
    assert_eq!(r.unclosed.len(), 1, "orphaned {{ should appear as unclosed, got: {:?}", r.unclosed);
    assert_eq!(r.unclosed[0].ch, '{');
}

#[test]
fn skip_forward_swapped_pair_gives_single_error() {
    // `({)}` — `(` then `{` open; `)` matches `(` via skip, orphaning `{`; `}` is extra.
    // Old: 2 mismatches. New: 0 user-facing mismatches + 1 unclosed `{`.
    let r = check_balance_str("({)}", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.mismatches.len(), 0, "extra-closer noise should be suppressed, got: {:?}", r.mismatches);
    assert_eq!(r.unclosed.len(), 1, "orphaned {{ expected, got: {:?}", r.unclosed);
    assert_eq!(r.unclosed[0].ch, '{');
}

#[test]
fn skip_forward_deep_nesting_bounded_mismatches() {
    // `([{([)]}>)` — multiple swapped pairs deep; skip-forward should produce far fewer
    // mismatches than the old cascade (≤ number of genuinely extra closers).
    // [{]})  simplified: `[{(]})`
    //   [ { ( ] } )
    //   `]` looks for `[` → found at 0, drain `{`(1) and `(`(2) → orphaned:{, (. match [.
    //   `}` → stack empty → extra }.  `)` → extra ).
    // Result: extra-closer noise suppressed; orphaned openers remain unclosed.
    let r = check_balance_str("[{(]})", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.mismatches.len(), 0, "expected no user-facing mismatches, got: {:?}", r.mismatches);
    assert!(r.unclosed.len() >= 1, "should have orphaned openers");
}

#[test]
fn extra_closers_suppressed_when_unclosed_exist() {
    // `{])` produces two extra closers while `{` remains unclosed.
    // These extra closer messages are noise once we already know the file is structurally broken.
    let r = check_balance_str("{])", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 1);
    assert_eq!(r.unclosed[0].ch, '{');
    assert!(r.mismatches.is_empty(), "expected extra-closer noise suppression");
}

#[test]
fn broken_nested_2_ts_prioritizes_unclosed_over_extra_closer_noise() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("test_artifacts/broken_nested_2.ts");
    let r = check_balance_file(&path, DEFAULT_PAIRS).expect("should read broken_nested_2.ts");
    assert!(!r.is_balanced);
    assert!(!r.unclosed.is_empty(), "expected primary unclosed errors");
    assert!(r.mismatches.is_empty(), "expected extra-closer noise to be suppressed, got: {:?}", r.mismatches);
}

#[test]
fn fix_suggestion_none_when_skip_forward_orphans() {
    // `[{]` — `]` matches `[` via skip, `{` becomes orphaned unclosed.
    // Since skip-forward reassigned matches, appending closers won't fix the file.
    // fix_suggestion must be None.
    let r = check_balance_str("[{]", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 1);
    assert_eq!(r.unclosed[0].ch, '{');
    assert!(r.fix_suggestion.is_none(), "interleaved errors should not produce append fix, got: {:?}", r.fix_suggestion);
}

#[test]
fn fix_suggestion_none_when_mixed_orphaned_and_remaining() {
    // `([{]` — `]` matches `[` via skip, `{` orphaned; `(` still on stack.
    // Both `{` and `(` are unclosed, but skip-forward orphaning means
    // a simple append won't fix the file. fix_suggestion must be None.
    let r = check_balance_str("([{]", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 2, "both ( and {{ should be unclosed");
    assert!(r.fix_suggestion.is_none(), "mixed orphaned+remaining should not produce append fix, got: {:?}", r.fix_suggestion);
}

// ---------------------------------------------------------------------------
// Issue 4: context lines must not truncate important content past column 80
// ---------------------------------------------------------------------------

#[test]
fn format_report_shows_full_long_line() {
    // Build a line that is 110 chars long; the type-error char is near col 100.
    // The formatted report should include content well past column 80.
    let padding = "x".repeat(90);
    let src = format!("let a = {}[;", padding); // unclosed `[` deep in line
    let r = check_balance_str(&src, DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    let report = format_report("test.rs", &r);
    // The 90-char padding must appear fully in the report (not truncated at 80)
    assert!(
        report.contains(&padding),
        "report should show full line past col 80; padding not found in:\n{report}"
    );
}

#[test]
fn allman_brace_unclosed_context_uses_previous_nonblank_line() {
    let src = "class BrokenProcessor\n{\n    public void Process()\n    {\n        foreach (var item in items)\n        {\n";
    let r = check_balance_str(src, DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 3);
    assert!(r.unclosed[0].line_text.contains("class BrokenProcessor"));
    assert!(r.unclosed[1].line_text.contains("public void Process()"));
    assert!(r.unclosed[2].line_text.contains("foreach (var item in items)"));
}

#[test]
fn fix_suggestion_none_when_orphaned_plus_new_opener() {
    // Same-line edge case: orphan an opener, then open a new one on the same line.
    // Input walkthrough:
    //   `[` push seq1
    //   `{` push seq2
    //   `]` matches `[` via skip-forward, orphaning `{` (seq2)
    //   `(` push seq3
    // Unclosed are `{` and `(`, but `{` was orphaned by skip-forward.
    // Appending closers won't fix the interleaved error → fix_suggestion must be None.
    let r = check_balance_str("[{](", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert_eq!(r.unclosed.len(), 2);
    assert!(r.fix_suggestion.is_none(), "orphaned+new opener should not produce append fix, got: {:?}", r.fix_suggestion);
}

// ---------------------------------------------------------------------------
// Issue 3: new-language artifact files exist and are picked up by collect_files
// ---------------------------------------------------------------------------

#[test]
fn new_language_artifact_files_exist() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let artifacts = manifest.join("test_artifacts");
    for (name, expected_balanced) in &[
        ("valid_nested_1.cpp",   true),
        ("broken_nested_1.cpp",  false),
        ("valid_nested_1.go",    true),
        ("broken_nested_1.go",   false),
        ("valid_nested_1.cs",    true),
        ("broken_nested_1.cs",   false),
        ("valid_nested_1.ps1",   true),
        ("broken_nested_1.ps1",  false),
        ("valid_nested_1.css",   true),
        ("broken_nested_1.css",  false),
        ("valid_nested_1.scss",  true),
        ("broken_nested_1.scss", false),
    ] {
        let path = artifacts.join(name);
        assert!(path.exists(), "artifact file missing: {name}");
        let result = bracebalance::check_balance_file(
            &path,
            DEFAULT_PAIRS,
        ).expect("should be readable");
        assert_eq!(
            result.is_balanced, *expected_balanced,
            "{name}: expected is_balanced={expected_balanced}, got {}",
            result.is_balanced
        );
    }
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

// ---------------------------------------------------------------------------
// Fix-suggestion reliability: guarantee that if present, appending actually fixes
// ---------------------------------------------------------------------------

#[test]
fn fix_suggestion_when_present_actually_fixes_simple() {
    // Pure unclosed openers — no interleaving, no extra closers.
    // Appending the fix_suggestion must make the result balanced.
    for src in &["({", "([{", "{{{", "([", "[[[((({{{"] {
        let r = check_balance_str(src, DEFAULT_PAIRS);
        assert!(!r.is_balanced);
        let fix = r.fix_suggestion.as_ref().expect(&format!("{src}: should have fix"));
        let fixed = format!("{src}{fix}");
        let r2 = check_balance_str(&fixed, DEFAULT_PAIRS);
        assert!(r2.is_balanced, "appending fix should balance '{src}' → '{fixed}'");
    }
}

#[test]
fn fix_suggestion_absent_when_extra_closers_present() {
    // `{])` — `]` and `)` are extra closers with no matching openers,
    // `{` is unclosed. Extra closers are suppressed but appending `}`
    // would still leave the file broken.
    let r = check_balance_str("{])", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(r.fix_suggestion.is_none(),
        "should not emit fix when suppressed extra closers exist, got: {:?}",
        r.fix_suggestion);
}

#[test]
fn fix_suggestion_absent_report_shows_manual_message() {
    // When fix_suggestion is None but unclosed openers exist, the report
    // should explain that manual fixes are needed, NOT show ">>> FIX:".
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    let report = format_report("test", &r);
    assert!(!report.contains(">>> FIX:"),
        "report should not show append FIX when the fix is unreliable");
    assert!(report.contains("manual") || report.contains("Manual"),
        "report should mention manual fix when suggestion is unavailable:\n{report}");
}

#[test]
fn all_broken_artifacts_fix_guarantee() {
    // For every broken artifact file: if fix_suggestion is Some, appending
    // it to the file MUST produce a balanced result. Zero tolerance.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("test_artifacts");
    let all_files = collect_files(&[dir]);
    let broken_files: Vec<_> = all_files.into_iter().filter(|f| {
        f.file_name().unwrap().to_str().unwrap().starts_with("broken_")
    }).collect();
    assert!(!broken_files.is_empty(), "should find broken artifact files");

    for path in &broken_files {
        let content = std::fs::read_to_string(path).unwrap();
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
        let r = check_balance_str_ext(&content, DEFAULT_PAIRS, ext.as_deref());
        assert!(!r.is_balanced, "{}: should be broken", path.display());

        if let Some(fix) = &r.fix_suggestion {
            let fixed = format!("{content}{fix}");
            let r2 = check_balance_str_ext(&fixed, DEFAULT_PAIRS, ext.as_deref());
            assert!(r2.is_balanced,
                "{}: fix_suggestion '{}' did NOT make the file balanced",
                path.display(), fix);
        }
        // If fix_suggestion is None, that's fine — we don't guarantee a fix for complex cases.
    }
}

#[test]
fn dedup_locations_aggregates_repeated_line() {
    let r = check_balance_str("[[[[", DEFAULT_PAIRS);
    let locations = dedup_locations(&r);
    assert_eq!(locations.len(), 1, "single repeated line should dedupe to one location");
    assert_eq!(locations[0].line_num, 1);
    assert_eq!(locations[0].occurrences, 4);

    let report = format_report("inline", &r);
    assert!(report.contains("Affected locations (deduped)"));
    assert!(report.contains("Line 1 (x4)"), "expected dedup count in report:\n{report}");
}

#[test]
fn expanded_report_includes_suppressed_details() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(r.suppressed_extra_closers > 0);

    let concise = format_report("inline", &r);
    assert!(!concise.contains("[TRACE] Suppressed extra closer details:"));

    let expanded = format_report_expanded("inline", &r);
    assert!(expanded.contains("[TRACE] Suppressed extra closer details:"));
    assert!(expanded.contains("Extra '}'"), "expanded trace should include hidden extra closer detail:\n{expanded}");
}

#[test]
fn expanded_report_does_not_compact_unclosed_list() {
    let src = "{\n(\n[\n{\n(\n[\n{\n(\n[\n{\n(\n[\n";
    let r = check_balance_str(src, DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(r.unclosed.len() > 8, "fixture should create many unclosed openers");

    let expanded = format_report_expanded("inline", &r);
    assert!(
        !expanded.contains("more omitted") && !expanded.contains("Showing first"),
        "expanded report should show full unclosed list without compaction:\n{expanded}"
    );
    assert!(
        expanded.contains("Line 12:"),
        "expanded output should include deep unclosed entries:\n{expanded}"
    );
}

#[test]
fn structured_output_has_required_fields() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    let payload = build_structured_diagnostics("inline", &r, false);
    let value = json!(payload);

    for key in [
        "balanced",
        "unclosed",
        "extra_closers",
        "suppressed_count",
        "fix_suggestion",
        "locations",
    ] {
        assert!(value.get(key).is_some(), "missing key: {key} in {value}");
    }
}

#[test]
fn structured_output_expanded_includes_details_and_hints() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    let payload = build_structured_diagnostics("inline", &r, true);
    let value = json!(payload);

    assert!(value.get("extra_closer_details").and_then(|v| v.as_array()).is_some());
    assert!(value.get("unclosed_details").and_then(|v| v.as_array()).is_some());
    let hints = value
        .get("ranked_repair_hints")
        .and_then(|v| v.as_array())
        .expect("ranked_repair_hints should be an array");
    assert!(!hints.is_empty(), "expected ranked hints in structured output");
}

#[test]
fn structured_output_concise_uses_empty_arrays_not_null() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    let payload = build_structured_diagnostics("inline", &r, false);
    let value = json!(payload);

    let unclosed_details = value
        .get("unclosed_details")
        .and_then(|v| v.as_array())
        .expect("unclosed_details should be an array in concise mode");
    let extra_details = value
        .get("extra_closer_details")
        .and_then(|v| v.as_array())
        .expect("extra_closer_details should be an array in concise mode");

    assert!(unclosed_details.is_empty(), "concise details should be []");
    assert!(extra_details.is_empty(), "concise details should be []");
}

#[test]
fn structured_output_includes_primary_locations() {
    let r = check_balance_str("[{]}\n}\n", DEFAULT_PAIRS);
    let payload = build_structured_diagnostics("inline", &r, false);
    let value = json!(payload);

    let locations = value
        .get("locations")
        .and_then(|v| v.as_array())
        .expect("locations should be an array");
    let primary = value
        .get("primary_locations")
        .and_then(|v| v.as_array())
        .expect("primary_locations should be an array");

    assert!(!locations.is_empty(), "locations should not be empty");
    assert!(!primary.is_empty(), "primary_locations should not be empty");
    assert!(primary.len() <= locations.len(), "primary locations should be a focused subset");
}

#[test]
fn interleaved_case_has_actionable_fallback_when_fix_is_null() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    assert!(r.fix_suggestion.is_none(), "interleaved case should not have append fix");

    let payload = build_structured_diagnostics("inline", &r, false);
    let value = json!(payload);
    let fallback = value
        .get("fallback_fix_suggestion")
        .and_then(|v| v.as_str())
        .expect("fallback_fix_suggestion should be present for interleaved cases");

    assert!(!fallback.trim().is_empty(), "fallback fix guidance should be actionable");
    assert!(fallback.contains("line"), "fallback guidance should point to a location: {fallback}");
}

#[test]
fn primary_locations_is_prioritized_subset_for_complex_failures() {
    // Many locations; primary_locations should be a smaller, prioritized subset.
    let src = "{\n(\n[\n{\n(\n[\n{\n(\n[\n{\n";
    let r = check_balance_str(src, DEFAULT_PAIRS);
    assert!(!r.is_balanced);

    let payload = build_structured_diagnostics("inline", &r, false);
    let value = json!(payload);

    let locations = value
        .get("locations")
        .and_then(|v| v.as_array())
        .expect("locations should be an array");
    let primary = value
        .get("primary_locations")
        .and_then(|v| v.as_array())
        .expect("primary_locations should be an array");

    assert!(locations.len() > 6, "fixture should produce many locations");
    assert!(
        primary.len() < locations.len(),
        "primary_locations should be a focused subset; got primary={} locations={}",
        primary.len(),
        locations.len()
    );
}

#[test]
fn concise_report_compacts_large_unclosed_lists() {
    // Large unclosed set should be compacted in concise mode.
    let src = "{\n(\n[\n{\n(\n[\n{\n(\n[\n{\n(\n[\n";
    let r = check_balance_str(src, DEFAULT_PAIRS);
    assert!(!r.is_balanced);
    assert!(r.unclosed.len() > 8, "fixture should create a large unclosed list");

    let report = format_report("inline", &r);
    assert!(
        report.contains("Showing first") || report.contains("more omitted"),
        "concise report should compact large unclosed lists:\n{report}"
    );
}

#[test]
fn concise_report_uses_actionable_manual_fallback_when_no_append_fix() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    assert!(r.fix_suggestion.is_none(), "interleaved case should have no append fix");

    let report = format_report("inline", &r);
    assert!(
        report.contains("Manual repair: start at line"),
        "concise output should show actionable fallback guidance:\n{report}"
    );
}

#[test]
fn concise_report_includes_suppressed_detail_preview() {
    let r = check_balance_str("[{]}", DEFAULT_PAIRS);
    assert!(r.suppressed_extra_closers > 0, "fixture should create suppressed details");

    let report = format_report("inline", &r);
    assert!(
        report.contains("Suppressed detail preview"),
        "concise output should include suppressed detail preview:\n{report}"
    );
    assert!(
        report.contains("Use expanded diagnostics"),
        "concise output should point to expanded diagnostics for full trace:\n{report}"
    );
}

#[test]
fn all_broken_artifacts_have_actionable_fix_guidance() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("test_artifacts");
    let all_files = collect_files(&[dir]);
    let broken_files: Vec<_> = all_files
        .into_iter()
        .filter(|f| f.file_name().unwrap().to_str().unwrap().starts_with("broken_"))
        .collect();
    assert!(!broken_files.is_empty(), "should find broken files");

    for path in broken_files {
        let result = check_balance_file(&path, DEFAULT_PAIRS).expect("should read artifact");
        assert!(!result.is_balanced, "{} should be broken", path.display());

        let payload = build_structured_diagnostics(&path.display().to_string(), &result, false);
        assert!(
            payload.fix_suggestion.is_some() || payload.fallback_fix_suggestion.is_some(),
            "{} should provide append fix or actionable fallback guidance",
            path.display()
        );
    }
}
