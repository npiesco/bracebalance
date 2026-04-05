from __future__ import annotations

SEGMENTS = [
    {
        "id": "intro",
        "after_event": "prompt_sent",
        "fallback_event": None,
        "fallback_offset": 3.0,
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
        "fallback_event": "copilot_responding",
        "fallback_offset": 10.0,
        "offset": 0.5,
        "text": (
            "Copilot calls the bracebalance MCP tool to scan for unclosed "
            "brackets, parentheses, and curly braces."
        ),
    },
    {
        "id": "editing",
        "after_event": "edit_seen",
        "fallback_event": "mcp_tool_seen",
        "fallback_offset": 30.0,
        "offset": 0.5,
        "text": (
            "With the errors identified, Copilot edits the source file directly — "
            "inserting the missing closing brackets exactly where they belong."
        ),
    },
    {
        "id": "cli_verify",
        "after_event": "cli_output_seen",
        "fallback_event": "edit_seen",
        "fallback_offset": 8.0,
        "offset": 0.5,
        "text": (
            "Now it runs the command-line interface to verify the fix — "
            "the same structural check, this time from a terminal."
        ),
    },
    {
        "id": "done",
        "after_event": "balanced_proven",
        "fallback_event": "demo_done",
        "fallback_offset": -3.0,
        "offset": 0.5,
        "text": (
            "The CLI confirms the file is balanced. "
            "No manual debugging — Copilot found and fixed every structural "
            "error in one pass."
        ),
    },
]
