from __future__ import annotations

# One continuous narration that walks through the demo as it plays.
# ~280 words at ~2.2 wps ≈ 126s to match the sped-up video.
NARRATION_TEXT = (
    "Welcome to BraceBalance. "
    "BraceBalance is an MCP server and command-line tool that checks source files "
    "for mismatched braces, brackets, and parentheses. "
    "It integrates directly with GitHub Copilot, so the AI can detect and fix "
    "structural errors without any human intervention. "

    "Right now, Copilot is receiving a task: inspect a broken TypeScript file "
    "called broken review dot t s. "
    "The file has deliberate brace errors that simulate real-world mistakes "
    "a developer might leave behind during a refactor. "

    "Copilot's first move is to call the bracebalance MCP tool. "
    "This tool parses the file character by character, tracking every opening "
    "and closing delimiter. "
    "It skips string literals, template literals, and comments "
    "to avoid false positives. "
    "When it finds an unclosed brace, it reports the exact line and column number. "

    "With the errors identified, Copilot edits the source file directly. "
    "It inserts the missing closing brackets exactly where they belong. "
    "This is a single autonomous pass. No prompts, no confirmations, "
    "no back-and-forth. Copilot reads the diagnostics and applies the fix. "

    "Now Copilot runs the BraceBalance command-line interface to verify the repair. "
    "This is the same structural check, executed from the terminal "
    "as an independent confirmation. "

    "The CLI confirms the file is balanced. "
    "Every opening delimiter has a matching close. "
    "No manual debugging was needed. "
    "Copilot found and fixed every structural error in one pass. "

    "That's the full workflow. Prompt to fix, fully automated. "
    "BraceBalance runs as an MCP server, a standalone CLI tool, or a Rust library. "
    "It supports custom delimiter pairs and handles nested structures correctly. "
    "Check it out on GitHub."
)
