# BraceBalance

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/npiesco/bracebalance/actions/workflows/rust.yml/badge.svg)](https://github.com/npiesco/bracebalance/actions)

A high-signal brace/bracket balance checker in Rust with a fast CLI, language-aware sanitizer, and an MCP server for editor/agent integrations.

## Features

- **Reliable Pair Checking**: Detects unclosed openers, extra closers, and interleaved/mismatched nesting across `()`, `{}`, `[]`, and optional `<>`.
- **Language-Aware Sanitizer**: Strips comments and string literals before checking so bracket-like text inside literals/comments does not produce false positives.
- **Actionable Repair Guidance**:
  - Safe append fix when guaranteed (`fix_suggestion`)
  - Manual fallback guidance when append fix is unsafe (`fallback_fix_suggestion`)
- **Noise-Reduced Diagnostics**:
  - Skip-forward mismatch recovery
  - Deduped locations with occurrence counts
  - Prioritized primary locations for triage
  - Concise mode with compaction and suppressed-detail preview
  - Expanded mode with full suppressed trace and ranked hints
- **MCP Server**: First-class tools for `check_text`, `check_path`, and `check_paths` over stdio.
- **Structured Output**: Optional JSON diagnostics for automation and CI pipelines.
- **CI-Friendly Exit Codes**: Returns `0` when all checks pass and `1` when any check fails.

## Key Dependencies

| Crate | Purpose |
|---|---|
| [clap](https://crates.io/crates/clap) | CLI argument parsing |
| [rmcp](https://crates.io/crates/rmcp) | MCP server framework/transport |
| [serde](https://crates.io/crates/serde) | Structured diagnostics serialization |
| [serde_json](https://crates.io/crates/serde_json) | JSON diagnostics output |
| [tokio](https://crates.io/crates/tokio) | Async runtime for MCP server |
| [schemars](https://crates.io/crates/schemars) | MCP parameter schema generation |
| [nom](https://crates.io/crates/nom) | Parser primitives used in sanitizer/runtime paths |

## Quick Start

### Prerequisites

- Rust (latest stable)

### Running Locally

```bash
git clone https://github.com/npiesco/bracebalance
cd bracebalance
cargo build --release

# CLI binary
target/release/bracebalance src/

# MCP server binary
target/release/bracebalance-mcp-server
```

### Basic CLI Usage

```bash
# Default pairs: () {} []
bracebalance src/main.rs

# Recursive scan (supported extensions only)
bracebalance src/

# Custom pairs
bracebalance -p "()" -p "[]" script.py

# Include angle brackets
bracebalance --all include/*.hpp
```

### Configuration

Configure pair selection via CLI flags (`clap`):

```bash
# Default pair set
bracebalance src/

# Custom pair set
bracebalance -p "{}" -p "[]" config.json

# Full common set
bracebalance --all src/
```

Configure MCP output behavior per tool call:

- `pairs`: custom list such as `["()", "{}"]`
- `all_pairs`: boolean
- `output`: `"text"` (default) or `"json"`
- `diagnostics_level`: `"concise"` (default) or `"expanded"`

## MCP Integration

### VS Code

Add this to `.vscode/mcp.json`:

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

### Claude Desktop

```json
{
  "mcpServers": {
    "bracebalance": {
      "command": "bracebalance-mcp-server"
    }
  }
}
```

### rmcp Runtime Note

For rmcp 0.16 servers, `.waiting()` is required to keep the process alive:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyMcpServer::new();
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
```

Also: avoid writing protocol output to stdout from an MCP stdio server. Use stderr logging (`eprintln!`) for diagnostics.

## Development

### Testing

Run all tests:

```bash
cargo test
```

Fixture coverage in `test_artifacts/` includes deeply nested valid and broken examples across multiple language families, used for both classification and diagnostics quality checks.

### Architecture

BraceBalance is organized around three layers:

- **Sanitizer layer**: extension-aware stripping of comments/strings before structural checks.
- **Balance engine**: skip-forward recovery, unclosed/opener tracking, suppression handling, and fix/fallback guidance generation.
- **Presentation layer**: concise/expanded text rendering and structured JSON diagnostics for MCP/automation consumers.

### Diagnostics Semantics (Safe Fix Contract)

- `fix_suggestion` is emitted only when append-at-EOF is guaranteed safe.
- When append safety cannot be guaranteed, `fallback_fix_suggestion` provides actionable manual steps.
- In concise JSON mode, `unclosed_details` and `extra_closer_details` are arrays (`[]`) rather than `null`.

### Supported Extension Families

Directory scans include TypeScript/JavaScript, systems languages, JVM languages, scripting languages, Go/Swift/Dart, .NET, shell, SQL/GraphQL, config/data formats, markup, stylesheets, infra/build files, and editor scripts (full extension set is defined in `SUPPORTED_EXTENSIONS` in `src/lib.rs`).

## License

MIT.

## Contributing

Contributions are welcome. Open an issue or pull request with:

- the expected vs actual behavior,
- a minimal reproduction,
- test evidence (`cargo test` output),
- and MCP/CLI command examples where relevant.
