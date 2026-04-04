#!/usr/bin/env python3

import os
import subprocess
import sys
import time
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
START_SCRIPT = ROOT_DIR / "demo" / "scripts" / "start_copilot_session.py"
OPEN_TERMINAL = ROOT_DIR / "demo" / "scripts" / "open_agent_terminal.py"


def run_capture(*args: str) -> str:
    result = subprocess.run(args, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def require_command(name: str) -> None:
    if not run_capture("which", name):
        print(f"missing required command: {name}", file=sys.stderr)
        sys.exit(1)


def get_window_pid(window_id: str) -> str:
    return run_capture("xdotool", "getwindowpid", window_id)


def get_window_name(window_id: str) -> str:
    return run_capture("xdotool", "getwindowname", window_id)


def wait_for_target_window(pid: int, timeout_seconds: float) -> str:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        output = run_capture("xdotool", "search", "--onlyvisible", "--pid", str(pid))
        windows = [line.strip() for line in output.splitlines() if line.strip()]
        if windows:
            return windows[-1]
        time.sleep(0.1)
    return ""


def activate_window(window_id: str) -> None:
    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def capture_window_screenshot(window_id: str, screenshot_path: Path) -> None:
    screenshot_path.parent.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        ["import", "-window", window_id, str(screenshot_path)],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or f"failed to capture screenshot for window {window_id}")
    if not screenshot_path.is_file() or screenshot_path.stat().st_size == 0:
        raise RuntimeError(f"screenshot was not written: {screenshot_path}")


def main() -> int:
    require_command("xfce4-terminal")
    require_command("xdotool")
    require_command("import")
    require_command("uv")

    if "DISPLAY" not in os.environ or not os.environ["DISPLAY"]:
        print("DISPLAY is not set; cannot launch a GUI terminal from this environment.", file=sys.stderr)
        return 1

    if not START_SCRIPT.is_file():
        print(f"start script not found: {START_SCRIPT}", file=sys.stderr)
        return 1

    launch_wait_seconds = float(os.environ.get("LAUNCH_WAIT_SECONDS", "10"))
    screenshot_path = Path(os.environ.get("DEMO_LAUNCH_SCREENSHOT", "/tmp/bracebalance-demo-launch.png"))

    process = subprocess.Popen(
        [
            "uv",
            "run",
            "python",
            str(OPEN_TERMINAL),
            "bash",
            "-lc",
            "DEMO_WINDOW_ID=$(xdotool getactivewindow) uv run python ./demo/scripts/start_copilot_session.py; exec bash",
        ],
        cwd=ROOT_DIR,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    time.sleep(0.5)

    target_window = wait_for_target_window(process.pid, launch_wait_seconds)
    if not target_window:
        print(f"unable to detect a terminal window for pid {process.pid} after launch", file=sys.stderr)
        return 1

    activate_window(target_window)
    time.sleep(0.5)
    capture_window_screenshot(target_window, screenshot_path)

    target_window_pid = get_window_pid(target_window) or "unknown"
    target_window_name = get_window_name(target_window) or "unknown"
    print(
        f"Launched terminal window {target_window} pid={target_window_pid} "
        f"name={target_window_name} screenshot={screenshot_path} running {START_SCRIPT}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
