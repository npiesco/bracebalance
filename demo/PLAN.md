# BraceBalance Demo Plan

## Goal

Produce a narrated demo video showing a human and an agentic AI using BraceBalance across two surfaces:

- CLI
- MCP-backed agent session

The demo should feel like a realistic code-review and repair workflow, not a synthetic benchmark. It should use deterministic fixture files derived from the existing regression corpus.

## Demo Claim

BraceBalance gives both humans and agents a reliable way to detect and fix delimiter-structure problems in real source files, with:

- source-aware sanitization
- readable diagnostics
- CLI workflow support
- MCP workflow support for structured agent integration

## Output

The final deliverable should be a narrated MP4 showing:

1. a human inspecting a broken file
2. the CLI surfacing the issue
3. a real Copilot-style terminal session using MCP-backed tooling
4. the agent applying or suggesting a repair
5. the human reviewing the diff
6. the clean rerun
7. a directory-level scan
8. a second language pass to prove breadth

## Surfaces To Capture

- Terminal window for CLI commands and test reruns
- Agent terminal window showing a real Copilot-style session
- Editor window showing source, edit, and diff

Optional:

- Git diff view if that is cleaner than in-editor diff

The recording should avoid trying to show the entire desktop. The capture should target the active window or a fixed tiled layout.

## Agent Session Model

The MCP surface should not be shown as a raw inspector. Use a real terminal session that the viewer can read.

`agent-tty` is reference material for how to drive a terminal-based demo session. It is not part of the visible BraceBalance demo surface.

Recommended approach:

- launch a terminal window directly with `xfce4-terminal`
- run the Copilot session command as the terminal's startup command
- use Copilot interactive mode with an initial prompt so the first task is deterministic

This keeps the demo focused on a visible human-and-agent workflow instead of a backend tool panel.

### Why This Is Better

- it looks like a real coding workflow
- the agent session is legible on screen
- MCP is still present, but it is shown through an actual agent interaction
- it avoids introducing a second terminal product into the BraceBalance demo

### Session Startup Requirement

The first visible prompt in the terminal should instruct Copilot exactly what to do:

```text
Inspect demo/fixtures/session1/broken_review.ts in this repo. Use the bracebalance MCP server tools first to identify the structural delimiter errors, then use the CLI to verify the diagnosis, apply the smallest correct fix, and confirm the file is balanced.
```

Recommended Copilot startup form:

```bash
copilot -i "Inspect demo/fixtures/session1/broken_review.ts in this repo. Use the bracebalance MCP server tools first to identify the structural delimiter errors, then use the CLI to verify the diagnosis, apply the smallest correct fix, and confirm the file is balanced."
```

If scripting is needed, treat terminal launch and command delivery as capture orchestration details. The footage should still look like a normal terminal session.

Recommended launch helper:

```bash
uv run python ./demo/scripts/launch_and_seed_terminal.py
```

That script should:

1. launch a clean `xfce4-terminal` window directly
2. pass the Copilot startup command directly to the terminal at launch
3. resolve the terminal window by PID with `xdotool search --pid`
4. capture a verification screenshot with `import -window`
5. leave the audience looking at a normal terminal session

Validated direct launch command:

```bash
xfce4-terminal --disable-server --working-directory=/home/npiesco/bracebalance --title='BraceBalance Demo' -x bash -lc 'uv run python ./demo/scripts/start_copilot_session.py; exec bash'
```

This direct `xfce4-terminal` path is the one that has been validated. `exo-open` indirection is not the recommended launch path for the demo.

### Reference Only

Use the local `agent-tty` README only to borrow ideas for:

- launching and controlling a terminal session reproducibly
- delivering the first prompt deterministically
- sequencing a terminal-based recording
- locating the real terminal window by PID for screenshots
- using `Ctrl+C` and `Escape` as interruption or dismissal primitives when needed

Do not show `agent-tty` itself in the BraceBalance video.

## Demo Fixtures

Do not edit files inside `test_artifacts/` during recording. Copy a small subset into a demo-only fixture directory.

Create:

- `demo/fixtures/session1/broken_review.ts`
- `demo/fixtures/session1/broken_review.rs`
- `demo/fixtures/session1/mixed/valid_sample.ts`
- `demo/fixtures/session1/mixed/broken_sample.ts`

Suggested source copies:

- `demo/fixtures/session1/broken_review.ts`
  source: `test_artifacts/broken_nested_1.ts`
- `demo/fixtures/session1/broken_review.rs`
  source: `test_artifacts/broken_nested_1.rs`
- `demo/fixtures/session1/mixed/valid_sample.ts`
  source: `test_artifacts/valid_nested_1.ts`
- `demo/fixtures/session1/mixed/broken_sample.ts`
  source: `test_artifacts/broken_nested_2.ts`

If the copied TypeScript or Rust fixtures are visually too long for the recording, create shorter demo variants derived from them. Keep the same failure pattern:

- comments and strings containing misleading delimiters
- one real unclosed structure
- a repair that is visible and fast to explain

## Recommended Story

### Scene 1: Open The Broken File

Human opens `demo/fixtures/session1/broken_review.ts` in the editor.

Purpose:

- establish that the file looks like real code
- show that the error is not trivially obvious by eye

### Scene 2: CLI Check

Run:

```bash
cargo run --bin bracebalance -- demo/fixtures/session1/broken_review.ts
```

Purpose:

- show the CLI is the fastest human-facing entrypoint
- surface the primary location and fix guidance

Expected outcome:

- non-zero exit
- readable diagnostics

### Scene 3: Agent Session Starts

The agent session should be visible in a real terminal window, not an MCP inspector.

Recommended sequence:

1. launch a terminal window
2. `cd` into the repo root
3. launch Copilot with the initial prompt in interactive mode

Recommended opening prompt:

```text
Inspect demo/fixtures/session1/broken_review.ts in this repo. Use the bracebalance MCP server tools first to identify the structural delimiter errors, then use the CLI to verify the diagnosis, apply the smallest correct fix, and confirm the file is balanced.
```

Purpose:

- show a real agent workflow instead of a debug surface
- make MCP feel integrated into the agent session rather than separate from it

Expected outcome:

- the agent inspects the file, uses BraceBalance-backed tooling, and proposes or applies the repair

### Scene 4: Agent Repairs The File

The agent edits the copied demo fixture in the editor.

The repair should be intentionally small and visible:

- close the missing array or object literal
- close the missing function or object block

Avoid a giant multi-location edit. The viewer should be able to follow the fix in one glance.

### Scene 5: Human Reviews The Diff

Show a concise diff of the edited file.

Purpose:

- make the collaboration model explicit
- show that the agent did not blindly rewrite the file

### Scene 6: Clean CLI Rerun

Run:

```bash
cargo run --bin bracebalance -- demo/fixtures/session1/broken_review.ts
```

Expected outcome:

- exit 0
- balanced report

This is the payoff for the first arc.

### Scene 7: Directory Scan

Run:

```bash
cargo run --bin bracebalance -- demo/fixtures/session1/mixed
```

Purpose:

- show the repo-scale or batch-check workflow
- demonstrate that BraceBalance can scan recursively across supported files

Expected outcome:

- mixed result because one file is intentionally still broken

### Scene 8: Cross-Language Proof

Run either CLI or MCP against:

```text
demo/fixtures/session1/broken_review.rs
```

Purpose:

- prove the tool is not TypeScript-specific
- reinforce language-aware sanitization and parser-independent structure checking

This step should be short. It is a breadth proof, not the main story.

## Exact Command Set

Assuming the repo root is the working directory:

```bash
# build binaries first for smooth demo playback
cargo build

# CLI check on the broken TypeScript file
cargo run --bin bracebalance -- demo/fixtures/session1/broken_review.ts

# rerun after the repair
cargo run --bin bracebalance -- demo/fixtures/session1/broken_review.ts

# recursive scan on a mixed directory
cargo run --bin bracebalance -- demo/fixtures/session1/mixed

# second-language proof point
cargo run --bin bracebalance -- demo/fixtures/session1/broken_review.rs
```

If avoiding `cargo run` startup noise is important for the recording, prefer built binaries:

```bash
target/debug/bracebalance demo/fixtures/session1/broken_review.ts
target/debug/bracebalance demo/fixtures/session1/mixed
target/debug/bracebalance demo/fixtures/session1/broken_review.rs
```

## MCP Role In The Demo

The MCP layer still matters, but it should be demonstrated through the agent session instead of through a raw tool console.

The BraceBalance MCP server already exposes:

- `check_text`
- `check_path`
- `check_paths`

The agent can use those tools as part of its workflow while the audience sees:

- the prompt
- the reasoning-visible terminal interaction
- the resulting edit and verification

If a backstage script is needed, it can still prepare or launch the MCP server separately. That is implementation detail, not primary footage.

## Narration Beats

Keep the narration factual and short. The narration should explain why each surface exists.

Suggested sequence:

1. "This file looks like ordinary application code, but one structural error is enough to break the parser."
2. "On the command line, BraceBalance checks the file without being distracted by braces inside comments or strings."
3. "The same analysis is also available through MCP, so the agent can work from structured diagnostics inside a real terminal session."
4. "The agent applies a minimal repair to the copied demo fixture."
5. "A quick diff keeps the human in control of what changed."
6. "Running the CLI again confirms the file is structurally balanced."
7. "The same binary also scans directories recursively, which makes it practical for repo-level checks."
8. "And because the checker is language-aware rather than parser-bound to one stack, the same workflow applies to Rust as well."

## Visual Plan

Use a fixed sequence of shots:

1. Editor full-screen with broken TypeScript file
2. Terminal full-screen for the first CLI run
3. Agent terminal full-screen for the Copilot session startup and first prompt
4. Editor full-screen for the repair
5. Diff view full-screen
6. Terminal full-screen for the clean rerun
7. Terminal full-screen for the mixed-directory scan
8. Terminal or agent terminal full-screen for the Rust proof point

Avoid rapid window switching during a single shot. Record separate segments and stitch them later.

## Recording Strategy

Use the same high-level model as the Duckcells demo:

1. preflight checks
2. scripted interaction run
3. segmented capture
4. narration synthesis
5. final mux

Recommended segment boundaries:

- `seg_01_open_file`
- `seg_02_cli_failure`
- `seg_03_agent_session_start`
- `seg_04_agent_edit`
- `seg_05_diff_review`
- `seg_06_cli_success`
- `seg_07_dir_scan`
- `seg_08_rust_check`

This gives flexibility to retake only the segment that fails.

## Dry Run Gates

Before recording, run a gated dry run:

```bash
uv run python ./demo/scripts/run_demo.py --dry-run
```

`--dry-run` means the full operational flow still runs, including the visible terminal launch and the Copilot session path. It only skips recording and audio/narration concerns.

When launched from a normal shell, `--dry-run` should hand itself off into a visible terminal first so the operator can see:

1. every hard gate run live
2. each pass/fail result with timing
3. the final Copilot session launch in that same visible flow

Each phase must hard-pass before the next begins:

1. reset fixtures
2. build CLI and MCP binaries
3. verify broken TypeScript CLI failure
4. verify mixed-directory CLI failure
5. verify Rust breadth CLI failure
6. verify Copilot MCP config contains `bracebalance`
7. verify Copilot startup prompt explicitly prioritizes `bracebalance` MCP tools
8. run a non-editing Copilot dry run against the broken TypeScript fixture
9. verify GUI launch prerequisites for the visible terminal session

If any phase fails, stop and fix that phase before recording.

The real demo uses the same entrypoint without the flag:

```bash
uv run python ./demo/scripts/run_demo.py
```

That run must pass the same gates first, then launch the visible terminal session. Both modes should emit timing for each phase, with explicit timing around the agent/MCP gate.

## Determinism Requirements

The demo run should be deterministic.

Requirements:

- Always start from clean copies in `demo/fixtures/session1/`
- Avoid editing regression fixtures in `test_artifacts/`
- Prebuild binaries before recording
- Use fixed terminal size, font size, and editor zoom
- Use stable working directory and relative paths
- Keep timestamps, notifications, and unrelated desktop noise off screen

## Prep Checklist

- Create demo fixture directory from copied regression files
- Build `bracebalance`
- Build `bracebalance-mcp-server`
- Verify the terminal-launch command opens the desired emulator
- Verify Copilot can be launched inside the terminal emulator
- Verify the first prompt can be delivered reproducibly
- Verify the terminal launches directly into the Copilot session command
- Verify all commands use the copied demo fixtures
- Verify the TypeScript repair yields exit 0 after edit
- Verify the mixed directory still fails before its own repair
- Verify the Rust fixture produces a clear diagnostic
- Verify the capture layout is legible at video resolution

## MVP Scope

If the first version needs to be shorter, cut it to:

1. broken TypeScript file
2. CLI failure
3. agent session startup
4. repair
5. clean rerun

Then add directory scan and Rust breadth as v2.

## Non-Goals

- Do not turn the demo into a full coding benchmark
- Do not show a long autonomous edit session
- Do not rely on live improvisation during recording
- Do not mutate the canonical regression corpus

## Next Build Steps

1. Create `demo/fixtures/session1/` by copying the selected files from `test_artifacts/`
2. Add a small script to reset the fixture directory before each recording
3. Add a demo runner that launches the terminal emulator, starts Copilot, and injects the opening prompt
4. Add capture hooks per scene
5. Add narration text and final assembly
