/// BraceBalance MCP Server
///
/// Exposes brace/bracket balance checking over the Model Context Protocol.
/// Tools:      check_text, check_path, check_paths
/// Transport:  stdio (pipe to an MCP client such as Claude Desktop)
use std::path::PathBuf;

use bracebalance::{
    build_structured_diagnostics, check_balance_file, check_balance_str_ext, collect_files,
    format_report, format_report_expanded, format_summary, resolve_pairs,
};
use serde_json::json;
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
    /// File extension hint (e.g. "rs", "py", "ts") for language-aware
    /// sanitization of string literals and comments.  When omitted, no
    /// sanitization is performed.
    pub ext: Option<String>,
    /// Custom pairs to check, e.g. ["()", "{}"] — default: () {} []
    pub pairs: Option<Vec<String>>,
    /// When true, check all built-in pairs: () {} [] <>
    pub all_pairs: Option<bool>,
    /// Output format: "text" (default) or "json".
    pub output: Option<String>,
    /// Diagnostics verbosity: "concise" (default) or "expanded".
    pub diagnostics_level: Option<String>,
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
    /// Output format: "text" (default) or "json".
    pub output: Option<String>,
    /// Diagnostics verbosity: "concise" (default) or "expanded".
    pub diagnostics_level: Option<String>,
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
    /// Output format: "text" (default) or "json".
    pub output: Option<String>,
    /// Diagnostics verbosity: "concise" (default) or "expanded".
    pub diagnostics_level: Option<String>,
}

fn parse_output_mode(value: Option<&str>) -> &'static str {
    match value.map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "json" => "json",
        _ => "text",
    }
}

fn parse_expanded(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.to_ascii_lowercase()).as_deref(),
        Some("expanded")
    )
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
        let output_mode = parse_output_mode(p.output.as_deref());
        let expanded = parse_expanded(p.diagnostics_level.as_deref());
        let result = check_balance_str_ext(&p.text, &pairs, p.ext.as_deref());
        if output_mode == "json" {
            json!(build_structured_diagnostics(label, &result, expanded)).to_string()
        } else if expanded {
            format_report_expanded(label, &result)
        } else {
            format_report(label, &result)
        }
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

        let output_mode = parse_output_mode(p.output.as_deref());
        let expanded = parse_expanded(p.diagnostics_level.as_deref());
        let path = PathBuf::from(&p.path);
        let files = collect_files(&[path]);
        run_checks(&files, &pairs, output_mode, expanded)
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

        let output_mode = parse_output_mode(p.output.as_deref());
        let expanded = parse_expanded(p.diagnostics_level.as_deref());
        let paths: Vec<PathBuf> = p.paths.iter().map(PathBuf::from).collect();
        let files = collect_files(&paths);
        run_checks(&files, &pairs, output_mode, expanded)
    }
}

// ---------------------------------------------------------------------------
// Delegate tool dispatch to the generated router
// ---------------------------------------------------------------------------

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BraceBalanceMcp {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        let description =
            "Check balanced brace/bracket pairs in source text, files, or directories.";

        rmcp::model::InitializeResult::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(
            rmcp::model::Implementation::new("bracebalance", env!("CARGO_PKG_VERSION"))
                .with_description(description),
        )
        .with_instructions(description)
    }
}

// ---------------------------------------------------------------------------
// Shared check runner (DRY)
// ---------------------------------------------------------------------------

/// Run checks on all files, collect output, return combined report + summary.
fn run_checks(
    files: &[PathBuf],
    pairs: &[(char, char)],
    output_mode: &str,
    expanded: bool,
) -> String {
    let mut output = String::new();
    let mut failed_files: Vec<PathBuf> = Vec::new();
    let mut checked = 0;
    let mut json_files = Vec::new();

    for filepath in files {
        if !filepath.exists() {
            if output_mode == "json" {
                json_files.push(json!({
                    "label": filepath.display().to_string(),
                    "error": format!("File not found: {}", filepath.display())
                }));
            } else {
                output.push_str(&format!(
                    "[WARNING] File not found: {}\n",
                    filepath.display()
                ));
            }
            continue;
        }
        checked += 1;
        match check_balance_file(filepath, pairs) {
            Ok(result) => {
                let is_ok = result.is_balanced;
                if output_mode == "json" {
                    json_files.push(json!(build_structured_diagnostics(
                        &filepath.display().to_string(),
                        &result,
                        expanded,
                    )));
                } else if expanded {
                    output.push_str(&format_report_expanded(
                        &filepath.display().to_string(),
                        &result,
                    ));
                } else {
                    output.push_str(&format_report(&filepath.display().to_string(), &result));
                }
                if !is_ok {
                    failed_files.push(filepath.clone());
                }
            }
            Err(e) => {
                if output_mode == "json" {
                    json_files.push(json!({
                        "label": filepath.display().to_string(),
                        "error": e
                    }));
                } else {
                    output.push_str(&format!("[ERROR] {e}\n"));
                }
                failed_files.push(filepath.clone());
            }
        }
    }

    if output_mode == "json" {
        json!({
            "checked": checked,
            "failed": failed_files.len(),
            "failed_files": failed_files
                .iter()
                .map(|f| f.display().to_string())
                .collect::<Vec<_>>(),
            "files": json_files,
        })
        .to_string()
    } else {
        if files.len() > 1 {
            output.push_str(&format_summary(checked, &failed_files));
        }

        output
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = BraceBalanceMcp::new();
    let transport = stdio();

    // IMPORTANT: `.serve()` only starts the MCP service — it returns
    // immediately.  You MUST call `.waiting().await` on the returned
    // `RunningService` to keep the process alive and handling requests.
    // Without it the server exits instantly and the MCP client sees
    // "Connection state: Stopped".
    //
    // Correct:
    //   let service = server.serve(transport).await?;
    //   service.waiting().await?;
    //
    // Wrong — exits immediately:
    //   server.serve(transport).await?;
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
