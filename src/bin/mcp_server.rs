/// BraceBalance MCP Server
///
/// Exposes brace/bracket balance checking over the Model Context Protocol.
/// Tools:      check_text, check_path, check_paths
/// Transport:  stdio (pipe to an MCP client such as Claude Desktop)
use std::path::PathBuf;

use bracebalance::{
    check_balance_file, check_balance_str, collect_files, format_report, format_summary,
    resolve_pairs,
};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

/// Parameters for checking raw text content.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CheckTextParams {
    /// The raw source text to check.
    pub text: String,
    /// Optional display label (e.g. a filename) used in the report.
    pub label: Option<String>,
    /// Custom pairs to check, e.g. ["()", "{}"] — default: () {} []
    pub pairs: Option<Vec<String>>,
    /// When true, check all built-in pairs: () {} [] <>
    pub all_pairs: Option<bool>,
}

/// Parameters for checking a single file or scanning a directory.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CheckPathParams {
    /// Absolute or relative path to a file or directory. Directories are
    /// scanned recursively for files with supported extensions.
    pub path: String,
    /// Custom pairs to check, e.g. ["()", "{}"] — default: () {} []
    pub pairs: Option<Vec<String>>,
    /// When true, check all built-in pairs: () {} [] <>
    pub all_pairs: Option<bool>,
}

/// Parameters for checking multiple files or directories.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CheckPathsParams {
    /// One or more absolute or relative paths. Directories are scanned
    /// recursively. Duplicate paths are deduplicated automatically.
    pub paths: Vec<String>,
    /// Custom pairs to check, e.g. ["()", "{}"] — default: () {} []
    pub pairs: Option<Vec<String>>,
    /// When true, check all built-in pairs: () {} [] <>
    pub all_pairs: Option<bool>,
}

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

/// The BraceBalance MCP server.
#[derive(Debug, Clone)]
pub struct BraceBalanceMcp {
    tool_router: ToolRouter<Self>,
}

impl BraceBalanceMcp {
    /// Create a new server instance.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for BraceBalanceMcp {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[tool_router]
impl BraceBalanceMcp {
    /// Check that paired characters are balanced in a string of source code.
    ///
    /// Returns a detailed report including mismatch locations, unclosed openers,
    /// and a suggested fix if needed.
    #[tool(
        name = "check_text",
        description = "Check that brace/bracket pairs are balanced in raw source text."
    )]
    pub async fn check_text(&self, params: Parameters<CheckTextParams>) -> String {
        let p = params.0;
        let pairs = match resolve_pairs(
            &p.pairs.unwrap_or_default(),
            p.all_pairs.unwrap_or(false),
        ) {
            Ok(pairs) => pairs,
            Err(e) => return format!("[ERROR] {e}"),
        };

        let label = p.label.as_deref().unwrap_or("<inline text>");
        let result = check_balance_str(&p.text, &pairs);
        format_report(label, &result)
    }

    /// Check that paired characters are balanced in a single file or directory.
    ///
    /// When a directory is given it is scanned recursively for supported source
    /// file extensions. Returns a concatenated report followed by a summary.
    #[tool(
        name = "check_path",
        description = "Check brace/bracket balance for a file or recursively scan a directory."
    )]
    pub async fn check_path(&self, params: Parameters<CheckPathParams>) -> String {
        let p = params.0;
        let pairs = match resolve_pairs(
            &p.pairs.unwrap_or_default(),
            p.all_pairs.unwrap_or(false),
        ) {
            Ok(pairs) => pairs,
            Err(e) => return format!("[ERROR] {e}"),
        };

        let path = PathBuf::from(&p.path);
        let files = collect_files(&[path]);
        run_checks(&files, &pairs)
    }

    /// Check that paired characters are balanced across multiple files or directories.
    ///
    /// Directories are scanned recursively. Returns a concatenated report for
    /// every file followed by an overall summary.
    #[tool(
        name = "check_paths",
        description = "Check brace/bracket balance across multiple files or directories."
    )]
    pub async fn check_paths(&self, params: Parameters<CheckPathsParams>) -> String {
        let p = params.0;
        let pairs = match resolve_pairs(
            &p.pairs.unwrap_or_default(),
            p.all_pairs.unwrap_or(false),
        ) {
            Ok(pairs) => pairs,
            Err(e) => return format!("[ERROR] {e}"),
        };

        let paths: Vec<PathBuf> = p.paths.iter().map(PathBuf::from).collect();
        let files = collect_files(&paths);
        run_checks(&files, &pairs)
    }
}

// ---------------------------------------------------------------------------
// Delegate tool dispatch to the generated router
// ---------------------------------------------------------------------------

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BraceBalanceMcp {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        rmcp::model::InitializeResult {
            server_info: rmcp::model::Implementation {
                name: "bracebalance".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some(
                    "Check balanced brace/bracket pairs in source text, files, or directories."
                        .to_string(),
                ),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Check balanced brace/bracket pairs in source text, files, or directories."
                    .to_string(),
            ),
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Shared check runner (DRY)
// ---------------------------------------------------------------------------

/// Run checks on all files, collect output, return combined report + summary.
fn run_checks(files: &[PathBuf], pairs: &[(char, char)]) -> String {
    let mut output = String::new();
    let mut failed_files: Vec<PathBuf> = Vec::new();
    let mut checked = 0;

    for filepath in files {
        if !filepath.exists() {
            output.push_str(&format!(
                "[WARNING] File not found: {}\n",
                filepath.display()
            ));
            continue;
        }
        checked += 1;
        match check_balance_file(filepath, pairs) {
            Ok(result) => {
                let is_ok = result.is_balanced;
                output.push_str(&format_report(&filepath.display().to_string(), &result));
                if !is_ok {
                    failed_files.push(filepath.clone());
                }
            }
            Err(e) => {
                output.push_str(&format!("[ERROR] {e}\n"));
                failed_files.push(filepath.clone());
            }
        }
    }

    if files.len() > 1 {
        output.push_str(&format_summary(checked, &failed_files));
    }

    output
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = BraceBalanceMcp::new();
    let transport = stdio();
    server.serve(transport).await?;
    Ok(())
}
