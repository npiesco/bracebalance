#!/usr/bin/env python3

import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
COMPLETION_MARKER = os.environ.get("DEMO_DONE_MARKER", "BRACEBALANCE_DEMO_DONE_9F1C7E")
PROMPT = (
    "Inspect demo/fixtures/session1/broken_review.ts in this repo. "
    "Step 1: Use the bracebalance MCP server check_path tool on the file with default pairs (do not set all_pairs, do not check <>). "
    "Step 2: Verify with the CLI by running exactly: cargo run --quiet --bin bracebalance -- demo/fixtures/session1/broken_review.ts "
    "(no -p flag needed — default pairs are () {} []). "
    "Step 3: Apply the smallest correct fix to each error the tools report, then re-run the CLI to confirm the file is balanced. "
    "Do not ask follow-up questions; make reasonable assumptions and keep going. "
    "Do not inspect comments, headers, strings, template literals, angle brackets, or secondary hypotheses. "
    "If both the MCP tool and the CLI say the file is balanced for () {} [], treat the task as complete immediately. "
    "If Copilot presents approval or confirmation prompts, choose the option that allows continuing safely in this trusted repo. "
    f"When the work is complete, print exactly {COMPLETION_MARKER} on its own line, then print a short final summary."
)
ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
COMPLETION_LINE_RE = re.compile(r"(?m)^\s*(?:DEMO_DONE|" + re.escape(COMPLETION_MARKER) + r")\s*$")

ARTIFACTS_DIR = ROOT_DIR / "test_artifacts"
FIXTURES_DIR = ROOT_DIR / "demo" / "fixtures" / "session1"
COPILOT_CONFIG = Path.home() / ".copilot" / "config.json"


def strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text).replace("\r", "")


def visible_prelude() -> None:
    start_delay = float(os.environ.get("DEMO_START_DELAY_SECONDS", "4"))
    prompt_delay = float(os.environ.get("DEMO_PROMPT_DELAY_SECONDS", "3"))
    print()
    print("BraceBalance demo session")
    print(f"Repo: {ROOT_DIR}")
    print("Mode: visible")
    print(f"Starting Copilot in {start_delay:g}s...")
    time.sleep(start_delay)
    print()
    print("Task for Copilot:")
    print(PROMPT)
    print()
    print(f"Launching agent in {prompt_delay:g}s...")
    time.sleep(prompt_delay)
    print()


def run_capture(*args: str) -> str:
    result = subprocess.run(args, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def get_window_id() -> str:
    configured = os.environ.get("DEMO_WINDOW_ID", "").strip()
    if configured:
        return configured
    if not os.environ.get("DISPLAY"):
        return ""
    return run_capture("xdotool", "getactivewindow")


def send_keys(window_id: str, *keys: str) -> None:
    if not window_id:
        return
    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", *keys],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def type_text(window_id: str, text: str) -> None:
    if not window_id:
        return
    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", text],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def terminate_process_group(process: subprocess.Popen[bytes], sig: int) -> None:
    try:
        os.killpg(process.pid, sig)
    except ProcessLookupError:
        return


def reset_fixtures() -> None:
    """Restore demo fixtures to their intentionally-broken state."""
    mapping = {
        "broken_nested_1.ts": FIXTURES_DIR / "broken_review.ts",
        "broken_nested_1.rs": FIXTURES_DIR / "broken_review.rs",
        "valid_nested_1.ts": FIXTURES_DIR / "mixed" / "valid_sample.ts",
        "broken_nested_2.ts": FIXTURES_DIR / "mixed" / "broken_sample.ts",
    }
    for src_name, dest in mapping.items():
        src = ARTIFACTS_DIR / src_name
        if src.is_file():
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)
    print("[controller] reset demo fixtures", file=sys.stderr)


def ensure_folder_trust() -> None:
    """Add the repo folder to Copilot's trusted_folders so the trust dialog is skipped."""
    folder = str(ROOT_DIR)
    try:
        data = json.loads(COPILOT_CONFIG.read_text()) if COPILOT_CONFIG.is_file() else {}
    except (json.JSONDecodeError, OSError):
        data = {}
    trusted = data.get("trusted_folders", [])
    if not isinstance(trusted, list):
        trusted = []
    if folder not in trusted:
        trusted.append(folder)
        data["trusted_folders"] = trusted
        COPILOT_CONFIG.parent.mkdir(parents=True, exist_ok=True)
        COPILOT_CONFIG.write_text(json.dumps(data, indent=2) + "\n")
        print(f"[controller] added {folder} to Copilot trusted_folders", file=sys.stderr)
    else:
        print("[controller] folder already trusted", file=sys.stderr)


def main() -> int:
    os.chdir(ROOT_DIR)
    reset_fixtures()
    ensure_folder_trust()

    if os.environ.get("DEMO_VISIBLE_MODE") == "1":
        visible_prelude()

    max_runtime_seconds = float(os.environ.get("DEMO_MAX_RUNTIME_SECONDS", "240"))
    idle_nudge_seconds = float(os.environ.get("DEMO_IDLE_NUDGE_SECONDS", "25"))
    window_id = get_window_id()

    copilot_command = [
        "copilot",
        "--screen-reader",
        "--no-mouse",
        "--allow-all-tools",
        "--allow-all-paths",
        "--allow-all-urls",
        "--no-ask-user",
        "--add-dir",
        str(ROOT_DIR),
    ]
    with tempfile.NamedTemporaryFile(prefix="bracebalance-copilot-", suffix=".log", delete=False) as handle:
        log_path = Path(handle.name)
    keep_log = os.environ.get("DEMO_KEEP_LOG") == "1"
    if keep_log:
        print(f"[controller] keeping transcript at {log_path}", file=sys.stderr)

    script_command = [
        "script",
        "-qefc",
        shlex.join(copilot_command),
        str(log_path),
    ]
    process = subprocess.Popen(
        script_command,
        cwd=str(ROOT_DIR),
        env={**os.environ, "DEMO_DONE_MARKER": COMPLETION_MARKER},
        start_new_session=True,
    )

    start_time = time.time()
    last_seen_len = 0
    last_output_at = start_time
    trusted_sent = False
    trust_answer_pos = 0
    generic_confirm_sent = 0
    prompt_sent = False
    copilot_responded = False
    response_start_pos = 0
    finish_nudge_sent = False
    nudge_sent = 0
    demo_done_seen_at: float | None = None
    exit_sent = False
    recent = ""

    # Phase timing events for narration alignment
    timings: dict[str, float] = {}
    timings_path = ROOT_DIR / "demo" / "output" / "timings.json"
    timings_path.parent.mkdir(parents=True, exist_ok=True)

    def record_timing(event: str) -> None:
        if event not in timings:
            timings[event] = time.time() - start_time
            timings_path.write_text(json.dumps(timings, indent=2) + "\n")
            print(f"\n[controller] timing: {event} @ {timings[event]:.1f}s", file=sys.stderr)

    try:
        while True:
            if process.poll() is not None:
                break

            now = time.time()
            if now - start_time > max_runtime_seconds:
                terminate_process_group(process, signal.SIGINT)
                print("\n[controller] timeout; sent SIGINT", file=sys.stderr)
                break

            if log_path.exists():
                content = log_path.read_text(errors="ignore")
                if len(content) > last_seen_len:
                    appended = content[last_seen_len:]
                    last_seen_len = len(content)
                    last_output_at = now
                    recent += strip_ansi(appended)
                    if len(recent) > 20000:
                        trim = len(recent) - 20000
                        recent = recent[trim:]
                        trust_answer_pos = max(0, trust_answer_pos - trim)
                        response_start_pos = max(0, response_start_pos - trim)

            recent_for_ready = recent[trust_answer_pos:]
            ready_for_prompt = (
                "Describe a task to get started." in recent_for_ready
                or "Type @ to mention files" in recent_for_ready
            )
            if ready_for_prompt and not prompt_sent:
                record_timing("prompt_sent")
                type_text(window_id, PROMPT)
                send_keys(window_id, "Return")
                prompt_sent = True
                last_output_at = now
                print("\n[controller] sent initial task prompt", file=sys.stderr)
                time.sleep(0.8)
                continue

            if (
                "Confirm folder trust" in recent
                or "Do you trust the files in this folder?" in recent
            ) and not trusted_sent:
                send_keys(window_id, "2", "Return")
                trusted_sent = True
                trust_answer_pos = len(recent)
                print("\n[controller] answered folder trust with option 2", file=sys.stderr)
                time.sleep(0.5)
                continue

            generic_prompt = (
                "Do you want to proceed?" in recent
                or "Quick safety check:" in recent
                or ("Allow" in recent and "1." in recent and "2." in recent)
                or "Continue?" in recent
            )
            if generic_prompt and generic_confirm_sent < 2:
                send_keys(window_id, "1", "Return")
                generic_confirm_sent += 1
                print("\n[controller] answered generic prompt with option 1", file=sys.stderr)
                time.sleep(0.5)
                continue

            if prompt_sent and not copilot_responded:
                response_markers = ("● Thinking", "● Search", "● Read", "● check_path",
                                    "● Finding", "● CLI brace", "● MCP tool",
                                    "Worked for", "─ Worked for")
                if any(m in recent for m in response_markers):
                    copilot_responded = True
                    response_start_pos = len(recent)
                    record_timing("copilot_responding")
                    print("\n[controller] Copilot started responding", file=sys.stderr)

            if not copilot_responded:
                time.sleep(0.2)
                continue

            response_text = recent[response_start_pos:]
            response_lower = response_text.lower()

            has_tool_results = (
                "(MCP: bracebalance)" in response_text
                or "UNCLOSED" in response_text
                or "[OK] BALANCED" in response_text
                or "lines read" in response_text
            )

            if "(MCP: bracebalance)" in response_text:
                record_timing("mcp_tool_seen")
            if "UNCLOSED" in response_text or "[OK] BALANCED" in response_text:
                record_timing("cli_output_seen")

            balanced_proven = (
                "[OK] BALANCED" in response_text
                and "(MCP: bracebalance)" in response_text
            )
            if balanced_proven:
                record_timing("balanced_proven")
            if balanced_proven and not finish_nudge_sent and demo_done_seen_at is None:
                type_text(
                    window_id,
                    f"Both checks are complete. Stop here. Print exactly {COMPLETION_MARKER} on its own line, then a short final summary, then exit.",
                )
                send_keys(window_id, "Return")
                finish_nudge_sent = True
                last_output_at = now
                print("\n[controller] sent finish nudge after balanced proof", file=sys.stderr)
                time.sleep(0.8)
                continue

            semantic_completion = (
                has_tool_results
                and ("No fix needed" in response_text or "no fix required" in response_lower
                     or "no fix was needed" in response_lower or "task complete" in response_lower
                     or "fixed" in response_lower)
                and "balanced" in response_lower
            )
            copilot_said_marker = f"\u25cf {COMPLETION_MARKER}" in response_text
            if has_tool_results and (copilot_said_marker or semantic_completion) and demo_done_seen_at is None:
                demo_done_seen_at = now
                record_timing("demo_done")
                print("\n[controller] detected completion", file=sys.stderr)

            if demo_done_seen_at is not None and not exit_sent and now - demo_done_seen_at >= 1.0:
                send_keys(window_id, "ctrl+c")
                time.sleep(0.5)
                send_keys(window_id, "ctrl+d")
                time.sleep(0.5)
                terminate_process_group(process, signal.SIGINT)
                exit_sent = True
                print("\n[controller] terminated Copilot session after completion", file=sys.stderr)
                time.sleep(1.0)
                continue

            if demo_done_seen_at is None and now - last_output_at >= idle_nudge_seconds and nudge_sent < 1:
                type_text(
                    window_id,
                    f"Continue with reasonable assumptions, stop further investigation, print exactly {COMPLETION_MARKER} on its own line, then a short final summary, then exit.",
                )
                send_keys(window_id, "Return")
                nudge_sent += 1
                last_output_at = now
                print("\n[controller] sent idle nudge", file=sys.stderr)

            time.sleep(0.2)

        try:
            return_code = process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            terminate_process_group(process, signal.SIGTERM)
            return_code = process.wait(timeout=10)
    finally:
        if not keep_log:
            try:
                log_path.unlink(missing_ok=True)
            except OSError:
                pass

    if demo_done_seen_at is None:
        print(
            f"\n[controller] completion was not observed (accepted: DEMO_DONE, {COMPLETION_MARKER}, or the balanced/no-fix summary)",
            file=sys.stderr,
        )
        return 1

    if return_code not in (0, 130, -2, -15):
        print(f"\n[controller] copilot exited with {return_code}", file=sys.stderr)
        return return_code

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
