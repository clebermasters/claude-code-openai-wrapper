/// Claude Agent SDK Tool Names
pub const CLAUDE_TOOLS: &[&str] = &[
    "Task",
    "Bash",
    "Glob",
    "Grep",
    "Read",
    "Edit",
    "Write",
    "NotebookEdit",
    "WebFetch",
    "TodoWrite",
    "WebSearch",
    "BashOutput",
    "KillShell",
    "Skill",
    "SlashCommand",
];

/// Default tools to allow when tools are enabled
pub const DEFAULT_ALLOWED_TOOLS: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "Bash",
    "Write",
    "Edit",
];

/// Tools to disallow by default (potentially dangerous or slow)
pub const DEFAULT_DISALLOWED_TOOLS: &[&str] = &[
    "Task",
    "WebFetch",
    "WebSearch",
];

/// Claude Models supported by Claude Agent SDK
pub const CLAUDE_MODELS: &[&str] = &[
    "claude-opus-4-5-20250929",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5-20251001",
    "claude-opus-4-1-20250805",
    "claude-opus-4-20250514",
    "claude-sonnet-4-20250514",
];

/// API defaults
pub const DEFAULT_MAX_TURNS: u32 = 10;
pub const DEFAULT_TIMEOUT_MS: u64 = 600_000;

/// Session management
pub const SESSION_CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes
pub const SESSION_MAX_AGE_SECS: u64 = 3600; // 1 hour

/// Version
pub const VERSION: &str = "2.2.0";
