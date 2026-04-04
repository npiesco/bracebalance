#!/usr/bin/env python3

import argparse
import json
import os
import shlex
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
DEMO_DIR = ROOT_DIR / "demo"
FIXTURES_DIR = DEMO_DIR / "fixtures" / "session1"
BROKEN_TS = FIXTURES_DIR / "broken_review.ts"
BROKEN_RS = FIXTURES_DIR / "broken_review.rs"
MIXED_DIR = FIXTURES_DIR / "mixed"
RESET_SCRIPT = DEMO_DIR / "scripts" / "reset_fixtures.py"
COPILOT_START = DEMO_DIR / "scripts" / "start_copilot_session.py"
GUI_LAUNCHER = DEMO_DIR / "scripts" / "launch_and_seed_terminal.py"
TTS_SCRIPT = DEMO_DIR / "scripts" / "synthesize_and_mux.py"
OPEN_TERMINAL = DEMO_DIR / "scripts" / "open_agent_terminal.py"
MCP_CONFIG = Path("/home/npiesco/.copilot/mcp-config.json")
COPILOT_CONFIG = Path("/home/npiesco/.copilot/config.json")
CLI_BIN = ROOT_DIR / "target" / "debug" / "bracebalance"
MCP_BIN = ROOT_DIR / "target" / "debug" / "bracebalance-mcp-server"

AGENT_PROMPT = (
    "In /home/npiesco/bracebalance, use the bracebalance MCP server tools first "
    "to inspect demo/fixtures/session1/broken_review.ts. Then summarize the "
    "delimiter errors in one paragraph. Do not edit any files and do not ask questions."
)
TRUSTED_FOLDER = str(ROOT_DIR)


@dataclass
class PhaseResult:
    name: str
    passed: bool
    detail: str
    duration_seconds: float


def run(
    args: list[str],
    *,
    cwd: Path | None = None,
    timeout: int = 120,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    return subprocess.run(
        args,
        cwd=str(cwd or ROOT_DIR),
        env=merged_env,
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )


def process_running(pattern: str) -> bool:
    result = run(["bash", "-lc", f"ps -ef | grep -F {shlex.quote(pattern)} | grep -v grep"], timeout=10)
    return result.returncode == 0 and bool(result.stdout.strip())


def phase(name: str):
    def decorator(fn):
        fn.phase_name = name
        return fn

    return decorator


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


@phase("reset-fixtures")
def phase_reset_fixtures() -> str:
    result = run(["uv", "run", "python", str(RESET_SCRIPT)], timeout=30)
    require(result.returncode == 0, result.stderr or result.stdout or "fixture reset failed")
    for path in [BROKEN_TS, BROKEN_RS, MIXED_DIR / "valid_sample.ts", MIXED_DIR / "broken_sample.ts"]:
        require(path.is_file(), f"missing fixture after reset: {path}")
    return "fixtures reset and present"


@phase("build-binaries")
def phase_build_binaries() -> str:
    result = run(["cargo", "build"], timeout=600)
    require(result.returncode == 0, result.stderr or result.stdout or "cargo build failed")
    require(CLI_BIN.is_file(), f"missing CLI binary: {CLI_BIN}")
    require(MCP_BIN.is_file(), f"missing MCP binary: {MCP_BIN}")
    return "cargo build succeeded; CLI and MCP binaries exist"


@phase("cli-broken-typescript")
def phase_cli_broken_typescript() -> str:
    result = run([str(CLI_BIN), str(BROKEN_TS)], timeout=30)
    require(result.returncode == 1, f"expected exit 1 for broken TS fixture, got {result.returncode}")
    require("UNCLOSED OPENER" in result.stdout, "missing unclosed opener diagnostics in CLI output")
    require("broken_review.ts" in result.stdout, "CLI output did not reference broken_review.ts")
    return "broken TypeScript fixture fails with structural diagnostics"


@phase("cli-mixed-directory")
def phase_cli_mixed_directory() -> str:
    result = run([str(CLI_BIN), str(MIXED_DIR)], timeout=30)
    require(result.returncode == 1, f"expected exit 1 for mixed directory, got {result.returncode}")
    require("valid_sample.ts" in result.stdout, "mixed directory output did not include valid_sample.ts")
    require("broken_sample.ts" in result.stdout, "mixed directory output did not include broken_sample.ts")
    require("SUMMARY" in result.stdout, "mixed directory output missing summary")
    return "mixed directory reports one broken file and one balanced file"


@phase("cli-rust-breadth")
def phase_cli_rust_breadth() -> str:
    result = run([str(CLI_BIN), str(BROKEN_RS)], timeout=30)
    require(result.returncode == 1, f"expected exit 1 for broken Rust fixture, got {result.returncode}")
    require("broken_review.rs" in result.stdout, "CLI output did not reference broken_review.rs")
    require("UNCLOSED OPENER" in result.stdout, "Rust output missing unclosed opener diagnostics")
    return "broken Rust fixture fails with clear diagnostics"


@phase("copilot-mcp-config")
def phase_copilot_mcp_config() -> str:
    require(MCP_CONFIG.is_file(), f"missing Copilot MCP config: {MCP_CONFIG}")
    data = json.loads(MCP_CONFIG.read_text())
    servers = data.get("mcpServers", {})
    bracebalance = servers.get("bracebalance")
    require(isinstance(bracebalance, dict), "bracebalance MCP server missing from Copilot config")
    require(
        bracebalance.get("command") == str(MCP_BIN),
        f"bracebalance MCP server points to {bracebalance.get('command')!r}, expected {MCP_BIN!s}",
    )
    return "Copilot config contains bracebalance MCP server entry"


@phase("copilot-folder-trust")
def phase_copilot_folder_trust() -> str:
    require(COPILOT_CONFIG.is_file(), f"missing Copilot config: {COPILOT_CONFIG}")
    data = json.loads(COPILOT_CONFIG.read_text())
    trusted_folders = data.get("trusted_folders")
    if trusted_folders is None:
        data["trusted_folders"] = [TRUSTED_FOLDER]
        status = "added"
    else:
        require(isinstance(trusted_folders, list), "trusted_folders is not a list in Copilot config")
        if TRUSTED_FOLDER not in trusted_folders:
            trusted_folders.append(TRUSTED_FOLDER)
            status = "added"
        else:
            status = "already-present"

    COPILOT_CONFIG.write_text(json.dumps(data, indent=2) + "\n")

    verified = json.loads(COPILOT_CONFIG.read_text())
    require(
        TRUSTED_FOLDER in verified.get("trusted_folders", []),
        f"{TRUSTED_FOLDER} was not persisted into Copilot trusted_folders",
    )
    return f"bracebalance folder trust {status} in Copilot config"


@phase("copilot-start-prompt")
def phase_copilot_start_prompt() -> str:
    content = COPILOT_START.read_text()
    require("bracebalance MCP" in content, "Copilot start prompt does not reference bracebalance MCP")
    require('"copilot"' in content, "Copilot start script is not launching Copilot")
    require('"-p"' not in content and "'-p'" not in content, "Copilot start script regressed to non-interactive prompt mode")
    require("sent initial task prompt" in content or "prompt_sent" in content or "type_text(window_id, PROMPT)" in content, "Copilot start script does not inject the initial task into the interactive session")
    require("--allow-all-tools" in content, "Copilot start script is not suppressing tool approval prompts")
    require("--no-ask-user" in content, "Copilot start script is not suppressing ask-user prompts")
    require("Confirm folder trust" in content, "Copilot start script does not handle folder trust prompts")
    require("COMPLETION_LINE_RE" in content or "DEMO_DONE" in content, "Copilot start script does not define completion detection")
    return "Copilot startup prompt uses interactive mode with controller-driven approvals and completion"


@phase("agent-dry-run")
def phase_agent_dry_run() -> str:
    command = [
        "copilot",
        "-p",
        AGENT_PROMPT,
        "--allow-all-tools",
        "--allow-all-paths",
        "--no-ask-user",
        "--add-dir",
        str(ROOT_DIR),
    ]
    process = subprocess.Popen(
        command,
        cwd=str(ROOT_DIR),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    saw_mcp_server = False
    agent_started_at = time.time()
    first_mcp_seen_at: float | None = None
    deadline = time.time() + 300

    try:
        while process.poll() is None:
            if process_running(str(MCP_BIN)):
                saw_mcp_server = True
                if first_mcp_seen_at is None:
                    first_mcp_seen_at = time.time()
            if time.time() >= deadline:
                process.send_signal(signal.SIGTERM)
                raise RuntimeError("Copilot dry run timed out")
            time.sleep(0.5)
    finally:
        stdout, stderr = process.communicate(timeout=30)

    require(process.returncode == 0, stderr or stdout or "Copilot dry run failed")
    combined = "\n".join(part for part in [stdout, stderr] if part)
    lowered = combined.lower()
    require("broken_review.ts" in combined, "Copilot output did not reference broken_review.ts")
    require("unclosed" in lowered or "delimiter" in lowered or "structural" in lowered, "Copilot output did not describe the delimiter problem")
    require(saw_mcp_server, "bracebalance-mcp-server was not observed during the Copilot dry run")
    total_duration = time.time() - agent_started_at
    if first_mcp_seen_at is not None:
        mcp_delay = first_mcp_seen_at - agent_started_at
        return (
            "Copilot completed a non-editing dry run and invoked bracebalance-mcp-server "
            f"(mcp_seen_at={mcp_delay:.2f}s, total={total_duration:.2f}s)"
        )
    return (
        "Copilot completed a non-editing dry run and invoked bracebalance-mcp-server "
        f"(total={total_duration:.2f}s)"
    )


@phase("gui-launch-prereqs")
def phase_gui_launch_prereqs() -> str:
    require(os.environ.get("DISPLAY"), "DISPLAY is not set")
    for command in ["xfce4-terminal", "xdotool", "import", "uv", "python3"]:
        result = run(["which", command], timeout=10)
        require(result.returncode == 0, f"missing required GUI/demo command: {command}")
    require(GUI_LAUNCHER.is_file(), f"missing Python launcher: {GUI_LAUNCHER}")
    return f"DISPLAY={os.environ['DISPLAY']} and GUI/demo launcher prerequisites are present"


PHASES = [
    phase_reset_fixtures,
    phase_build_binaries,
    phase_cli_broken_typescript,
    phase_cli_mixed_directory,
    phase_cli_rust_breadth,
    phase_copilot_mcp_config,
    phase_copilot_folder_trust,
    phase_copilot_start_prompt,
    phase_agent_dry_run,
    phase_gui_launch_prereqs,
]


def run_gates() -> tuple[bool, list[PhaseResult]]:
    results: list[PhaseResult] = []

    for fn in PHASES:
        name = fn.phase_name
        print(f"[RUN ] {name}")
        started_at = time.time()
        try:
            detail = fn()
        except Exception as exc:  # noqa: BLE001
            duration_seconds = time.time() - started_at
            results.append(PhaseResult(name=name, passed=False, detail=str(exc), duration_seconds=duration_seconds))
            print(f"[FAIL] {name} ({duration_seconds:.2f}s): {exc}")
            break
        else:
            duration_seconds = time.time() - started_at
            results.append(PhaseResult(name=name, passed=True, detail=detail, duration_seconds=duration_seconds))
            print(f"[PASS] {name} ({duration_seconds:.2f}s): {detail}")

    print("\nGate summary:")
    for result in results:
        status = "PASS" if result.passed else "FAIL"
        print(f"- {status} {result.name} ({result.duration_seconds:.2f}s): {result.detail}")

    if len(results) != len(PHASES):
        failed = next((result for result in results if not result.passed), None)
        if failed:
            print(f"\nHard stop at phase: {failed.name}", file=sys.stderr)
        return False, results

    print("\nAll gated phases passed.")
    return True, results


def launch_demo_session() -> int:
    print("\n[RUN ] launch-visible-demo-session")
    started_at = time.time()
    result = subprocess.run(
        ["uv", "run", "python", str(GUI_LAUNCHER)],
        cwd=str(ROOT_DIR),
        text=True,
        env={
            **os.environ,
            "DEMO_VISIBLE_MODE": "1",
            "DEMO_START_DELAY_SECONDS": os.environ.get("DEMO_START_DELAY_SECONDS", "4"),
            "DEMO_PROMPT_DELAY_SECONDS": os.environ.get("DEMO_PROMPT_DELAY_SECONDS", "3"),
        },
        check=False,
    )
    duration_seconds = time.time() - started_at
    if result.returncode != 0:
        print(f"[FAIL] launch-visible-demo-session ({duration_seconds:.2f}s): launcher exited {result.returncode}")
        return result.returncode
    print(f"[PASS] launch-visible-demo-session ({duration_seconds:.2f}s)")
    return 0


def launch_demo_session_in_current_terminal() -> int:
    print("\n[RUN ] launch-demo-session-in-current-terminal")
    started_at = time.time()
    result = subprocess.run(
        ["uv", "run", "python", str(COPILOT_START)],
        cwd=str(ROOT_DIR),
        text=True,
        env={
            **os.environ,
            "DEMO_VISIBLE_MODE": "1",
            "DEMO_START_DELAY_SECONDS": os.environ.get("DEMO_START_DELAY_SECONDS", "4"),
            "DEMO_PROMPT_DELAY_SECONDS": os.environ.get("DEMO_PROMPT_DELAY_SECONDS", "3"),
        },
        check=False,
    )
    duration_seconds = time.time() - started_at
    if result.returncode != 0:
        print(f"[FAIL] launch-demo-session-in-current-terminal ({duration_seconds:.2f}s): exited {result.returncode}")
        return result.returncode
    print(f"[PASS] launch-demo-session-in-current-terminal ({duration_seconds:.2f}s)")
    return 0


def launch_visible_dry_run_terminal() -> int:
    if not os.environ.get("DISPLAY"):
        print("DISPLAY is not set; cannot hand off dry run to a visible terminal.", file=sys.stderr)
        return 1

    result = subprocess.run(
        [
            "uv",
            "run",
            "python",
            str(OPEN_TERMINAL),
            "bash",
            "-lc",
            (
                f"cd {shlex.quote(str(ROOT_DIR))} && "
                "DEMO_VISIBLE_DRY_RUN=1 "
                "DEMO_WINDOW_ID=$(xdotool getactivewindow) "
                "uv run python ./demo/scripts/run_demo.py --dry-run; "
                "printf '\\nDry run terminal finished. Press Enter to close...\\n'; "
                "read -r _; "
                "exec bash"
            ),
        ],
        cwd=str(ROOT_DIR),
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(f"[FAIL] launch-visible-dry-run-terminal: launcher exited {result.returncode}")
        return result.returncode
    print("[PASS] launch-visible-dry-run-terminal")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the BraceBalance demo flow.")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Run the full gated flow visibly, but skip recording and audio stages.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.dry_run and os.environ.get("DEMO_VISIBLE_DRY_RUN") != "1":
        return launch_visible_dry_run_terminal()

    passed, _results = run_gates()
    if not passed:
        return 1

    if args.dry_run:
        print("\nDry run mode: executing full flow without recording or audio stages.")
        return launch_demo_session_in_current_terminal()

    rc = launch_demo_session()
    if rc != 0:
        return rc

    # Synthesize TTS narration and mux with recording
    print("\n[RUN ] synthesize-and-mux")
    started_at = time.time()
    tts_result = subprocess.run(
        ["uv", "run", "--with", "azure-cognitiveservices-speech", "python", str(TTS_SCRIPT)],
        cwd=str(ROOT_DIR),
        text=True,
        check=False,
    )
    duration_seconds = time.time() - started_at
    if tts_result.returncode != 0:
        print(f"[FAIL] synthesize-and-mux ({duration_seconds:.2f}s): exited {tts_result.returncode}")
        return tts_result.returncode
    print(f"[PASS] synthesize-and-mux ({duration_seconds:.2f}s)")

    final_video = DEMO_DIR / "output" / "bracebalance_demo_final.mp4"
    if final_video.is_file():
        size_mb = final_video.stat().st_size / (1024 * 1024)
        print(f"\n✓ Final video: {final_video} ({size_mb:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
