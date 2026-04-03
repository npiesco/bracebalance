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
RESET_SCRIPT = DEMO_DIR / "scripts" / "reset-fixtures.sh"
COPILOT_START = DEMO_DIR / "scripts" / "start-copilot-session.sh"
GUI_LAUNCHER = DEMO_DIR / "scripts" / "launch_and_seed_terminal.py"
MCP_CONFIG = Path("/home/npiesco/.copilot/mcp-config.json")
CLI_BIN = ROOT_DIR / "target" / "debug" / "bracebalance"
MCP_BIN = ROOT_DIR / "target" / "debug" / "bracebalance-mcp-server"

AGENT_PROMPT = (
    "In /home/npiesco/bracebalance, use the bracebalance MCP server tools first "
    "to inspect demo/fixtures/session1/broken_review.ts. Then summarize the "
    "delimiter errors in one paragraph. Do not edit any files and do not ask questions."
)


@dataclass
class PhaseResult:
    name: str
    passed: bool
    detail: str


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
    result = run([str(RESET_SCRIPT)], timeout=30)
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


@phase("copilot-start-prompt")
def phase_copilot_start_prompt() -> str:
    content = COPILOT_START.read_text()
    require("bracebalance MCP server tools first" in content, "Copilot start prompt does not prioritize bracebalance MCP")
    require("copilot -i" in content, "Copilot start script is not using interactive mode")
    return "Copilot startup prompt explicitly prefers bracebalance MCP first"


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
        "-s",
    ]
    process = subprocess.Popen(
        command,
        cwd=str(ROOT_DIR),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    saw_mcp_server = False
    deadline = time.time() + 300

    try:
        while process.poll() is None:
            if process_running(str(MCP_BIN)):
                saw_mcp_server = True
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
    return "Copilot completed a non-editing dry run and invoked bracebalance-mcp-server"


@phase("gui-launch-prereqs")
def phase_gui_launch_prereqs() -> str:
    require(os.environ.get("DISPLAY"), "DISPLAY is not set")
    for command in ["exo-open", "xdotool", "uv", "python3"]:
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
    phase_copilot_start_prompt,
    phase_agent_dry_run,
    phase_gui_launch_prereqs,
]


def run_gates() -> tuple[bool, list[PhaseResult]]:
    results: list[PhaseResult] = []

    for fn in PHASES:
        name = fn.phase_name
        print(f"[RUN ] {name}")
        try:
            detail = fn()
        except Exception as exc:  # noqa: BLE001
            results.append(PhaseResult(name=name, passed=False, detail=str(exc)))
            print(f"[FAIL] {name}: {exc}")
            break
        else:
            results.append(PhaseResult(name=name, passed=True, detail=detail))
            print(f"[PASS] {name}: {detail}")

    print("\nGate summary:")
    for result in results:
        status = "PASS" if result.passed else "FAIL"
        print(f"- {status} {result.name}: {result.detail}")

    if len(results) != len(PHASES):
        failed = next((result for result in results if not result.passed), None)
        if failed:
            print(f"\nHard stop at phase: {failed.name}", file=sys.stderr)
        return False, results

    print("\nAll gated phases passed.")
    return True, results


def launch_demo_session() -> int:
    print("\n[RUN ] launch-visible-demo-session")
    result = subprocess.run(
        ["uv", "run", "python", str(GUI_LAUNCHER)],
        cwd=str(ROOT_DIR),
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(f"[FAIL] launch-visible-demo-session: launcher exited {result.returncode}")
        return result.returncode
    print("[PASS] launch-visible-demo-session")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the BraceBalance demo flow.")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Run all hard pass/fail gates without launching the visible terminal session.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    passed, _results = run_gates()
    if not passed:
        return 1

    if args.dry_run:
        print("\nDry run requested; skipping visible terminal launch.")
        return 0

    return launch_demo_session()


if __name__ == "__main__":
    raise SystemExit(main())
