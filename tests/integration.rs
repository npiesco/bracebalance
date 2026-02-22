use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_bracebalance"))
}

fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_artifacts")
}

fn run_on(path: &std::path::Path) -> std::process::ExitStatus {
    Command::new(exe())
        .arg(path)
        .status()
        .unwrap_or_else(|e| panic!("failed to run bracebalance on {}: {e}", path.display()))
}

/// Every file whose name starts with "valid_" must exit 0 (balanced).
#[test]
fn valid_files_are_balanced() {
    let dir = artifacts_dir();
    let mut checked = 0;

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("test_artifacts directory not found")
        .map(|e| e.expect("dir entry error"))
        .filter(|e| e.file_name().to_string_lossy().starts_with("valid_"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let status = run_on(&entry.path());
        assert!(
            status.success(),
            "FAIL [{name}]: expected exit 0 (balanced), got exit {:?}",
            status.code()
        );
        checked += 1;
    }

    assert!(checked > 0, "no valid_* files found in test_artifacts/");
    println!("valid files checked: {checked}");
}

/// Every file whose name starts with "broken_" must exit 1 (errors detected).
#[test]
fn broken_files_are_detected() {
    let dir = artifacts_dir();
    let mut checked = 0;

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("test_artifacts directory not found")
        .map(|e| e.expect("dir entry error"))
        .filter(|e| e.file_name().to_string_lossy().starts_with("broken_"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let status = run_on(&entry.path());
        assert!(
            !status.success(),
            "FAIL [{name}]: expected exit 1 (errors), got exit 0 (false negative)"
        );
        checked += 1;
    }

    assert!(checked > 0, "no broken_* files found in test_artifacts/");
    println!("broken files checked: {checked}");
}

/// Passing a directory path scans all supported files recursively.
/// test_artifacts/ contains broken files, so the overall result must be exit 1.
#[test]
fn dir_scan_finds_broken_files() {
    let dir = artifacts_dir();
    let status = run_on(&dir);
    assert!(
        !status.success(),
        "expected exit 1 when scanning test_artifacts/ (contains broken files), got exit 0"
    );
}

/// A directory containing only balanced files must exit 0.
#[test]
fn dir_scan_all_valid_passes() {
    let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_tmp_valid");
    fs::create_dir_all(&tmp).expect("create tmp dir");

    // Copy every valid_* file from test_artifacts into it
    let dir = artifacts_dir();
    let entries: Vec<_> = fs::read_dir(&dir)
        .expect("test_artifacts not found")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("valid_"))
        .collect();

    assert!(!entries.is_empty(), "no valid_* files to copy");

    for entry in &entries {
        let dest = tmp.join(entry.file_name());
        fs::copy(entry.path(), &dest)
            .unwrap_or_else(|e| panic!("copy {} failed: {e}", entry.path().display()));
    }

    let status = run_on(&tmp);

    // Clean up
    let _ = fs::remove_dir_all(&tmp);

    assert!(
        status.success(),
        "expected exit 0 when scanning a directory of only valid files, got exit {:?}",
        status.code()
    );
}
