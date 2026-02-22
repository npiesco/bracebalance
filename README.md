# bracebalance

A fast CLI tool that checks source files for unbalanced paired characters — `{}`, `()`, `[]`, and optionally `<>`.

It tells you exactly which line has the problem, what was opened but never closed, and what you need to add to fix it.

## Features

- Detects mismatched openers/closers (e.g. `[` closed by `}`)
- Detects unclosed openers (e.g. `{` that has no matching `}`)
- Detects extra closers with no matching opener
- Reports the **fix** — lists exactly which closing characters to append and in what order
- Supports **any file type** — language-agnostic, works on `.py`, `.rs`, `.ts`, `.tsx`, `.js`, `.jsx`, `.json`, `.c`, `.go`, or anything else
- Supports **custom pairs** via `-p`
- Check **multiple files or directories** at once with a summary
- **MCP server** — expose all checks as tools to any MCP client (Claude Desktop, Cursor, etc.)
- CI-friendly: exits `0` if all files are balanced, `1` if any have errors

## Usage

```
bracebalance [OPTIONS] <FILES|DIRS>...
```

### Basic

```sh
# Check a single file (default pairs: () {} [])
bracebalance main.rs

# Check all supported files in a directory (recursive)
bracebalance src/

# Mix files and directories
bracebalance main.rs lib/ config.json

# Check multiple files by glob
bracebalance src/*.ts

# Check all common pairs including <>
bracebalance --all main.cpp

# Check only specific pairs
bracebalance -p "()" -p "[]" script.py
```

### Options

| Flag | Description |
|---|---|
| `-p <AB>` | Pairs to check, e.g. `"{}"` `"()"` `"[]"` `"<>"`. Repeatable. |
| `--all` | Check all common pairs: `()` `{}` `[]` `<>` |

Default pairs (when no flags are given): `()` `{}` `[]`

### Supported extensions (directory scan)

When given a directory, bracebalance recursively checks all files with these extensions:

| Category | Extensions |
|---|---|
| TypeScript / JavaScript | `.ts` `.tsx` `.js` `.jsx` `.mjs` `.cjs` |
| Systems | `.rs` `.c` `.cpp` `.cc` `.cxx` `.h` `.hpp` `.hxx` |
| JVM | `.java` `.kt` `.kts` `.scala` `.groovy` `.clj` `.cljs` `.cljc` |
| Scripting | `.py` `.rb` `.php` `.lua` `.r` `.pl` `.pm` |
| Go / Swift / Dart | `.go` `.swift` `.dart` |
| .NET | `.cs` `.fs` `.fsi` `.fsx` |
| Functional | `.hs` `.ml` `.mli` `.ex` `.exs` `.erl` `.hrl` `.elm` |
| Frontend frameworks | `.vue` `.svelte` |
| Shell | `.sh` `.bash` `.zsh` `.fish` `.ps1` `.psm1` |
| Data / query | `.sql` `.graphql` `.gql` `.proto` |
| Config / data | `.json` `.jsonc` `.toml` `.yml` `.yaml` |
| Markup | `.html` `.htm` `.xml` |
| Infra / build | `.tf` `.hcl` `.cmake` `.dockerfile` |
| Misc | `.vim` `.el` |

## Output

For a balanced file:

```
================================================================================
File: src/main.rs  |  Checking: () {} []
================================================================================
[OK] BALANCED: All () {} [] pairs match!
```

For a file with errors:

```
================================================================================
File: broken.py  |  Checking: () {} []
================================================================================

[ERROR] 2 issue(s) found:
  Line 12: '}' at line 12 does not match '[' opened at line 12
           {"key2": [4, 5, 6},
  Line 32: ']' at line 32 does not match '(' opened at line 32
           {"a": 8, "b": [9, (10, 11], ...}

[ERROR] 3 UNCLOSED OPENER(S):

These were NEVER closed:
  Line 4: '{' (needs '}')  data = {
  Line 5: '{' (needs '}')      "level1": {
  Line 6: '[' (needs ']')          "level2": [

>>> FIX: Add closing char(s): ]}} <<<

[FAILED] File has balance errors!
```

When checking multiple files, a summary is printed at the end:

```
================================================================================
SUMMARY
================================================================================
Total files checked: 8
Failed files: 2

[FAILED] Files with balance errors:
  - src/broken_a.ts
  - src/broken_b.json
```

## Installation

### From source

Requires [Rust](https://rustup.rs/).

```sh
git clone https://github.com/npiesco/bracebalance
cd bracebalance
cargo build --release
# CLI:        target/release/bracebalance
# MCP server: target/release/bracebalance-mcp-server
```

### Add to PATH

```sh
# Linux / macOS
cp target/release/bracebalance ~/.local/bin/
cp target/release/bracebalance-mcp-server ~/.local/bin/

# Windows (PowerShell)
Copy-Item .\target\release\bracebalance.exe $env:USERPROFILE\.cargo\bin\
Copy-Item .\target\release\bracebalance-mcp-server.exe $env:USERPROFILE\.cargo\bin\
```

## MCP Server

`bracebalance-mcp-server` speaks the [Model Context Protocol](https://modelcontextprotocol.io) over stdio — plug it into Claude Desktop, Cursor, or any other MCP host.

### Tools

| Tool | Description |
|---|---|
| `check_text` | Check raw source text supplied inline |
| `check_path` | Check a single file or recursively scan a directory |
| `check_paths` | Check multiple files/directories in one call |

All three tools accept optional `pairs` (e.g. `["()", "{}"]`) and `all_pairs` (boolean) to control which pairs are checked.

### Claude Desktop config

```json
{
  "mcpServers": {
    "bracebalance": {
      "command": "bracebalance-mcp-server"
    }
  }
}
```

## Running Tests

The `test_artifacts/` directory contains 21 deeply nested test files across 7 languages — 14 valid and 7 intentionally broken.

```sh
cargo test
```

```
test result: ok. 4 passed  (integration — file + dir scans)
test result: ok. 20 passed (library unit tests)
```

## Examples

```sh
# Scan an entire project directory
bracebalance src/

# Scan multiple directories
bracebalance src/ tests/ config/

# Check all TypeScript files in a project
bracebalance src/**/*.ts src/**/*.tsx

# Use in CI (non-zero exit on any error)
bracebalance --all src/*.c && echo "All good"

# Check JSON configs
bracebalance -p "{}" -p "[]" config.json

# Check angle brackets in C++ templates
bracebalance --all include/*.hpp
```
