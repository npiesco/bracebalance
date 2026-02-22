# bracebalance

A fast CLI tool that checks source files for unbalanced paired characters — `{}`, `()`, `[]`, and optionally `<>`.

It tells you exactly which line has the problem, what was opened but never closed, and what you need to add to fix it.

## Features

- Detects mismatched openers/closers (e.g. `[` closed by `}`)
- Detects unclosed openers (e.g. `{` that has no matching `}`)
- Detects extra closers with no matching opener
- Reports the **fix** — lists exactly which closing characters to append and in what order
- **Language-aware sanitizer** — strips comments and string literals before checking, so brackets inside strings/comments are never false-positives (see [Sanitizer](#language-aware-sanitizer) below)
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

## Language-aware sanitizer

For recognised file extensions, bracebalance strips comments and string literals from the source text before counting brackets. This prevents false positives from brackets that appear inside strings or comments.

| Language(s) | What is stripped |
|---|---|
| TypeScript / JavaScript / Vue / Svelte | `//` line, `/* */` block, `""` `''` `` `` `` strings |
| C / C++ / C# / Java / Go / Dart / Proto | `//` line, `/* */` block, `""` strings |
| Swift | `//` line, `/* */` block (nested), `"""` `""` `''` strings |
| Kotlin / Scala / Groovy | `//` line, `/* */` block, `"""` `""` `''` strings |
| Rust | `//` line, `/* */` block, raw `r#"..."#` strings |
| Python | `#` comment, `"""` `'''` triple-quoted, `r""` raw strings |
| Ruby | `#` comment, `"""` `""` `''` strings, `%q{}`/`%w[]` percent literals (nested), `<<~HEREDOC` heredocs |
| Shell / Bash / Zsh / Fish | `#` comment, `<<HEREDOC` heredocs, `""` `''` strings |
| PHP | `//` `#` `/* */` comments, `<<<EOT` heredocs |
| Elixir | `#` comment, `~r/.../` `~w{...}` sigils, `"""` `""` strings |
| Lua | `--` line, `--[[` block comments, `[[` long strings |
| SQL | `--` line comments, `$$`/`$tag$` dollar-quoting |
| GraphQL | `#` comment, `"""` doc-strings, `""` strings |
| Haskell | `--` line, `{- nested -}` block comments |
| OCaml / F# | `(* nested *)` block comments |
| Erlang | `%` line comments |
| Clojure | `;` line comments |
| Vim script | `"` line comments |
| HTML / XML | `<!-- -->` comments |
| TOML / YAML / R / etc. | `#` line comments |

For unsupported extensions the raw text is checked as-is (no sanitisation).

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

### VS Code config

Add to `.vscode/mcp.json`:

```jsonc
{
  "servers": {
    "bracebalance": {
      "type": "stdio",
      "command": "${workspaceFolder}/target/release/bracebalance-mcp-server.exe"
    }
  }
}
```

### rmcp MRE — `.waiting()` is required

If your rmcp 0.16 MCP server exits instantly ("Connection state: Stopped") the
moment a client connects, you are almost certainly missing the `.waiting()` call.

`.serve()` returns a `RunningService` future that **sets up** the connection — it
does **not** block.  You must call `.waiting().await` on the returned service to
keep the process alive and processing requests.

```rust
// ✅ Correct — server stays alive
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyMcpServer::new();
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;          // <── keeps the process alive
    Ok(())
}
```

```rust
// ❌ Wrong — exits immediately after handshake
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyMcpServer::new();
    let transport = rmcp::transport::stdio();
    server.serve(transport).await?;    // returns, process exits
    Ok(())
}
```

Also: **never** write to stdout (`println!`, `print!`, `dbg!`) in an MCP stdio
server — it corrupts the JSON-RPC framing and the client will disconnect.  Use
`eprintln!` (stderr) for diagnostics, or better yet use `tracing` with a stderr
subscriber.

## Running Tests

The `test_artifacts/` directory contains **51 deeply nested test files** across **16 languages** — 29 valid and 22 intentionally broken. They cover all sanitizer edge cases: nested comments, percent literals, heredocs, dollar-quoting, sigils, CRLF line endings, and more.

```sh
cargo test
```

```
test result: ok. 61 passed  (sanitizer unit tests)
test result: ok.  4 passed  (integration — file + dir scans)
test result: ok. 20 passed  (library unit tests)
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
