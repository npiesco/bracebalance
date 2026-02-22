# VS Code MCP Server Guide

## MCP Server Setup

Use this repo's MCP server in VS Code via stdio transport.

### Step 1: Build Release Binary

```sh
cargo build --release
# Produces:
#   target/release/bracebalance.exe      (CLI)
#   target/release/mcp_server.exe        (MCP server)
```

### Step 2: MCP Configuration

`.vscode/mcp.json` is already committed in this repo:

```json
{
  "servers": {
    "bracebalance": {
      "type": "stdio",
      "command": "${workspaceFolder}/target/release/mcp_server.exe"
    }
  }
}
```

**Note:** On Linux/macOS, change `mcp_server.exe` → `mcp_server`.

### Step 3: Trust and Start

1. Open the Chat view (`Ctrl+Alt+I`)
2. VS Code will prompt you to trust the MCP server — confirm it
3. The tools will appear in the tool picker

### Available MCP Tools

| Tool | What it does |
|---|---|
| `check_text` | Check raw source text supplied inline |
| `check_path` | Check a single file or recursively scan a directory |
| `check_paths` | Check multiple files/directories at once |

All tools accept optional `pairs` (e.g. `["()", "{}"]`) and `all_pairs` (boolean) to control which character pairs are checked. Default: `()` `{}` `[]`.

### Example prompts

```
Check if src/main.rs has balanced braces.

Scan the entire src/ directory for unbalanced brackets.

Check this snippet for bracket issues:
  fn foo() { let x = vec![1, 2, 3; }
```

## DO NOT

- Manually start the stdio server in a terminal — VS Code manages stdin/stdout automatically
