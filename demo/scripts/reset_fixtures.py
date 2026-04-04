#!/usr/bin/env python3

import shutil
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
FIXTURES = ROOT_DIR / "demo" / "fixtures" / "session1"
MIXED = FIXTURES / "mixed"
ARTIFACTS = ROOT_DIR / "test_artifacts"


def copy(src: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ARTIFACTS / src, dest)


def main() -> int:
    copy("broken_nested_1.ts", FIXTURES / "broken_review.ts")
    copy("broken_nested_1.rs", FIXTURES / "broken_review.rs")
    copy("valid_nested_1.ts", MIXED / "valid_sample.ts")
    copy("broken_nested_2.ts", MIXED / "broken_sample.ts")
    print(f"Reset demo fixtures in {FIXTURES}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
