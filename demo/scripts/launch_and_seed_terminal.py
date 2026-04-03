#!/usr/bin/env python3

import os
import subprocess
import sys
import time
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_COMMAND_FILE = ROOT_DIR / "demo" / "scripts" / "demo-session-command.txt"


def run_capture(*args: str) -> str:
    result = subprocess.run(args, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def require_command(name: str) -> None:
    if not run_capture("which", name):
        print(f"missing required command: {name}", file=sys.stderr)
        sys.exit(1)


def get_active_window() -> str:
    return run_capture("xdotool", "getactivewindow")


def list_visible_windows() -> list[str]:
    output = run_capture("xdotool", "search", "--onlyvisible", "--name", ".*")
    return sorted({line.strip() for line in output.splitlines() if line.strip()})


def get_window_pid(window_id: str) -> str:
    return run_capture("xdotool", "getwindowpid", window_id)


def get_window_name(window_id: str) -> str:
    return run_capture("xdotool", "getwindowname", window_id)


def wait_for_target_window(previous_window: str, before_windows: set[str], timeout_seconds: float) -> str:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        current_windows = set(list_visible_windows())
        new_windows = sorted(current_windows - before_windows)
        if new_windows:
            return new_windows[-1]

        current_window = get_active_window()
        if current_window and current_window != previous_window:
            return current_window

        time.sleep(0.1)

    fallback_window = get_active_window()
    if fallback_window:
        return fallback_window

    return ""


def type_line(window_id: str, line: str, type_delay_ms: int, post_line_delay_seconds: float) -> None:
    subprocess.run(["xdotool", "windowactivate", "--sync", window_id], check=True)
    if line:
        subprocess.run(
            ["xdotool", "type", "--window", window_id, "--delay", str(type_delay_ms), "--", line],
            check=True,
        )
    subprocess.run(["xdotool", "key", "--window", window_id, "Return"], check=True)
    time.sleep(post_line_delay_seconds)


def main() -> int:
    require_command("exo-open")
    require_command("xdotool")

    if "DISPLAY" not in os.environ or not os.environ["DISPLAY"]:
        print("DISPLAY is not set; cannot launch a GUI terminal from this environment.", file=sys.stderr)
        return 1

    command_file = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_COMMAND_FILE
    if not command_file.is_file():
        print(f"command file not found: {command_file}", file=sys.stderr)
        return 1

    launch_wait_seconds = float(os.environ.get("LAUNCH_WAIT_SECONDS", "6"))
    type_delay_ms = int(os.environ.get("TYPE_DELAY_MS", "1"))
    post_line_delay_seconds = float(os.environ.get("POST_LINE_DELAY_SECONDS", "0.2"))

    previous_window = get_active_window()
    before_windows = set(list_visible_windows())

    launcher = subprocess.Popen(
        [str(ROOT_DIR / "demo" / "scripts" / "open-agent-terminal.sh")],
        cwd=ROOT_DIR,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    target_window = wait_for_target_window(previous_window, before_windows, launch_wait_seconds)
    if not target_window:
        print("unable to detect a terminal window after launch", file=sys.stderr)
        return 1

    with command_file.open("r", encoding="utf-8") as handle:
        for raw_line in handle:
            type_line(target_window, raw_line.rstrip("\n"), type_delay_ms, post_line_delay_seconds)

    target_window_pid = get_window_pid(target_window) or "unknown"
    target_window_name = get_window_name(target_window) or "unknown"
    print(
        f"Seeded terminal window {target_window} pid={target_window_pid} "
        f"name={target_window_name} using {command_file}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
