#!/usr/bin/env python3

import os
import subprocess
import sys
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]


def main() -> int:
    title = os.environ.get("DEMO_TERMINAL_TITLE", "BraceBalance Demo")
    args = [
        "xfce4-terminal",
        "--disable-server",
        f"--working-directory={ROOT_DIR}",
        f"--title={title}",
    ]
    if len(sys.argv) > 1:
        args.extend(["-x", *sys.argv[1:]])
    result = subprocess.run(args, cwd=ROOT_DIR, check=False)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
