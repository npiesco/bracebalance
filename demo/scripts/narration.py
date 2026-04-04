from __future__ import annotations

SEGMENTS = [
    {
        "id": "intro",
        "after_event": "prompt_sent",
        "offset": 0.0,
        "text": (
            "BraceBalance ships an MCP server so Copilot can check any file "
            "for brace errors without leaving the editor. "
            "Here, Copilot receives a single task: inspect a broken TypeScript file."
        ),
    },
    {
        "id": "mcp_check",
        "after_event": "mcp_tool_seen",
        "offset": 0.5,
        "text": (
            "Copilot calls the bracebalance MCP tool to scan for unclosed "
            "brackets, parentheses, and curly braces."
        ),
    },
    {
        "id": "cli_verify",
        "after_event": "cli_output_seen",
        "offset": 0.5,
        "text": (
            "Now it verifies the results with the command-line interface, "
            "running the same structural check from a terminal."
        ),
    },
    {
        "id": "fix_and_done",
        "after_event": "balanced_proven",
        "offset": 0.5,
        "text": (
            "A few small edits later, the CLI confirms the file is balanced. "
            "No manual debugging — Copilot found and fixed every structural "
            "error in one pass."
        ),
    },
]
