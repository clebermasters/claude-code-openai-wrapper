# Claude Code CLI Complete Reference

> Compiled from official documentation at https://code.claude.com/docs/en/cli-reference,
> https://code.claude.com/docs/en/settings, https://code.claude.com/docs/en/permissions,
> https://code.claude.com/docs/en/mcp, https://code.claude.com/docs/en/model-config,
> and community sources. Last updated: 2026-03-19.

---

## Table of Contents

1. [Top-Level Commands](#1-top-level-commands)
2. [CLI Flags (Complete List)](#2-cli-flags-complete-list)
3. [Sub-Commands](#3-sub-commands)
4. [System Prompt Flags](#4-system-prompt-flags)
5. [Permission Modes](#5-permission-modes)
6. [Model Configuration](#6-model-configuration)
7. [Output Format Options](#7-output-format-options)
8. [Print Mode (Non-Interactive) Reference](#8-print-mode-non-interactive-reference)
9. [Environment Variables](#9-environment-variables)
10. [Settings.json Keys](#10-settingsjson-keys)
11. [MCP Sub-Commands](#11-mcp-sub-commands)
12. [Slash Commands (In-Session)](#12-slash-commands-in-session)
13. [Keyboard Shortcuts](#13-keyboard-shortcuts)
14. [Prompt Notation](#14-prompt-notation)

---

## 1. Top-Level Commands

| Command | Description | Example |
|---------|-------------|---------|
| `claude` | Start interactive session | `claude` |
| `claude "query"` | Start interactive session with initial prompt | `claude "explain this project"` |
| `claude -p "query"` | Non-interactive (print) mode, exit after response | `claude -p "explain this function"` |
| `cat file \| claude -p "query"` | Process piped content in print mode | `cat logs.txt \| claude -p "explain"` |
| `claude -c` | Continue most recent conversation in current directory | `claude -c` |
| `claude -c -p "query"` | Continue most recent conversation in print mode | `claude -c -p "Check for type errors"` |
| `claude -r "<session>" "query"` | Resume session by ID or name | `claude -r "auth-refactor" "Finish this PR"` |
| `claude update` | Update to latest version | `claude update` |
| `claude auth login` | Sign in to Anthropic account | `claude auth login --console` |
| `claude auth logout` | Log out from Anthropic account | `claude auth logout` |
| `claude auth status` | Show authentication status as JSON (exit 0=logged in, 1=not) | `claude auth status` |
| `claude agents` | List all configured subagents, grouped by source | `claude agents` |
| `claude mcp` | Configure Model Context Protocol servers | `claude mcp list` |
| `claude remote-control` | Start Remote Control server (no local interactive session) | `claude remote-control --name "My Project"` |
| `claude config list` | Display all settings | `claude config list` |
| `claude config get <key>` | Retrieve a specific setting | `claude config get model` |
| `claude config set <key> <value>` | Update a setting | `claude config set model opus` |

---

## 2. CLI Flags (Complete List)

### Session Control

| Flag | Short | Description | Example |
|------|-------|-------------|---------|
| `--print` | `-p` | Print response without interactive mode (non-interactive/SDK mode) | `claude -p "query"` |
| `--continue` | `-c` | Load the most recent conversation in the current directory | `claude -c` |
| `--resume` | `-r` | Resume a specific session by ID or name, or show interactive picker | `claude -r auth-refactor` |
| `--session-id` | | Use a specific session ID (must be valid UUID) | `claude --session-id "550e..."` |
| `--name` | `-n` | Set a display name for the session | `claude -n "my-feature-work"` |
| `--fork-session` | | When resuming, create a new session ID instead of reusing original | `claude --resume abc123 --fork-session` |
| `--from-pr` | | Resume sessions linked to a specific GitHub PR (number or URL) | `claude --from-pr 123` |
| `--no-session-persistence` | | Disable session persistence (print mode only) | `claude -p --no-session-persistence "query"` |
| `--worktree` | `-w` | Start in isolated git worktree | `claude -w feature-auth` |

### Model & Effort

| Flag | Description | Example |
|------|-------------|---------|
| `--model` | Set model: alias (`sonnet`, `opus`, `haiku`, `opusplan`, `sonnet[1m]`, `opus[1m]`) or full name | `claude --model claude-sonnet-4-6` |
| `--fallback-model` | Automatic fallback model when default is overloaded (print mode only) | `claude -p --fallback-model sonnet "query"` |
| `--effort` | Set effort level: `low`, `medium`, `high`, `max` (Opus 4.6 only). Session-scoped | `claude --effort high` |

### Output Control

| Flag | Description | Example |
|------|-------------|---------|
| `--output-format` | Output format for print mode: `text`, `json`, `stream-json` | `claude -p --output-format json "query"` |
| `--input-format` | Input format for print mode: `text`, `stream-json` | `claude -p --input-format stream-json` |
| `--include-partial-messages` | Include partial streaming events (requires `--print` + `--output-format=stream-json`) | `claude -p --output-format stream-json --include-partial-messages "query"` |
| `--json-schema` | Validated JSON output matching schema (print mode only) | `claude -p --json-schema '{"type":"object",...}' "query"` |
| `--verbose` | Enable verbose logging, full turn-by-turn output | `claude --verbose` |

### Limits

| Flag | Description | Example |
|------|-------------|---------|
| `--max-turns` | Limit number of agentic turns (print mode only). No limit by default | `claude -p --max-turns 3 "query"` |
| `--max-budget-usd` | Maximum dollar amount for API calls (print mode only) | `claude -p --max-budget-usd 5.00 "query"` |

### System Prompt

| Flag | Description | Example |
|------|-------------|---------|
| `--system-prompt` | Replace the entire system prompt with custom text | `claude --system-prompt "You are a Python expert"` |
| `--system-prompt-file` | Load system prompt from file, replacing default | `claude --system-prompt-file ./prompt.txt` |
| `--append-system-prompt` | Append custom text to the end of the default system prompt | `claude --append-system-prompt "Always use TypeScript"` |
| `--append-system-prompt-file` | Append file contents to default system prompt | `claude --append-system-prompt-file ./rules.txt` |

Note: `--system-prompt` and `--system-prompt-file` are mutually exclusive. Append flags can be combined with either.

### Permissions & Security

| Flag | Description | Example |
|------|-------------|---------|
| `--permission-mode` | Begin in a specified permission mode (`default`, `acceptEdits`, `plan`, `dontAsk`, `bypassPermissions`) | `claude --permission-mode plan` |
| `--dangerously-skip-permissions` | Skip permission prompts (use with caution) | `claude --dangerously-skip-permissions` |
| `--allow-dangerously-skip-permissions` | Enable permission bypassing as option without activating. Composable with `--permission-mode` | `claude --permission-mode plan --allow-dangerously-skip-permissions` |
| `--permission-prompt-tool` | MCP tool to handle permission prompts in non-interactive mode | `claude -p --permission-prompt-tool mcp_auth_tool "query"` |

### Tool Control

| Flag | Description | Example |
|------|-------------|---------|
| `--allowedTools` | Tools that execute without prompting. Supports pattern matching | `claude --allowedTools "Bash(git log *)" "Read"` |
| `--disallowedTools` | Tools removed from model's context entirely | `claude --disallowedTools "Bash(rm *)" "Edit"` |
| `--tools` | Restrict which built-in tools Claude can use. `""` = none, `"default"` = all | `claude --tools "Bash,Edit,Read"` |

### Agents & Extensions

| Flag | Description | Example |
|------|-------------|---------|
| `--agent` | Specify an agent for the session (overrides `agent` setting) | `claude --agent my-custom-agent` |
| `--agents` | Define custom subagents dynamically via JSON | `claude --agents '{"reviewer":{"description":"Reviews code","prompt":"..."}}'` |
| `--add-dir` | Add additional working directories (validates paths exist) | `claude --add-dir ../apps ../lib` |
| `--mcp-config` | Load MCP servers from JSON files or strings (space-separated) | `claude --mcp-config ./mcp.json` |
| `--strict-mcp-config` | Only use MCP servers from `--mcp-config`, ignore all others | `claude --strict-mcp-config --mcp-config ./mcp.json` |
| `--plugin-dir` | Load plugins from directory (repeat for multiple) | `claude --plugin-dir ./my-plugins` |
| `--channels` | Enable named channel servers | `claude --channels plugin:fakechat@claude-plugins-official` |
| `--dangerously-load-development-channels` | Enable non-allowlisted channels for local dev | `claude --dangerously-load-development-channels server:webhook` |

### Chrome & IDE

| Flag | Description | Example |
|------|-------------|---------|
| `--chrome` | Enable Chrome browser integration | `claude --chrome` |
| `--no-chrome` | Disable Chrome browser integration | `claude --no-chrome` |
| `--ide` | Auto-connect to IDE on startup | `claude --ide` |

### Remote & Web

| Flag | Description | Example |
|------|-------------|---------|
| `--remote` | Create new web session on claude.ai | `claude --remote "Fix the login bug"` |
| `--remote-control` / `--rc` | Start interactive session with Remote Control enabled | `claude --remote-control "My Project"` |
| `--teleport` | Resume a web session in local terminal | `claude --teleport` |
| `--teammate-mode` | Agent team display mode: `auto`, `in-process`, `tmux` | `claude --teammate-mode in-process` |

### Settings & Configuration

| Flag | Description | Example |
|------|-------------|---------|
| `--settings` | Path to settings JSON file or JSON string | `claude --settings ./settings.json` |
| `--setting-sources` | Comma-separated list of setting sources: `user`, `project`, `local` | `claude --setting-sources user,project` |
| `--betas` | Beta headers for API requests (API key users only) | `claude --betas interleaved-thinking` |

### Initialization & Maintenance

| Flag | Description | Example |
|------|-------------|---------|
| `--init` | Run initialization hooks and start interactive mode | `claude --init` |
| `--init-only` | Run initialization hooks and exit (no interactive session) | `claude --init-only` |
| `--maintenance` | Run maintenance hooks and exit | `claude --maintenance` |

### Debug & Misc

| Flag | Description | Example |
|------|-------------|---------|
| `--debug` | Enable debug mode with optional category filtering | `claude --debug "api,mcp"` or `claude --debug "!statsig,!file"` |
| `--disable-slash-commands` | Disable all skills and commands for this session | `claude --disable-slash-commands` |
| `--version` / `-v` | Output the version number | `claude -v` |

---

## 3. Sub-Commands

### `claude auth`

| Sub-Command | Description | Flags |
|-------------|-------------|-------|
| `claude auth login` | Sign in to Anthropic account | `--email` (pre-fill email), `--sso` (force SSO), `--console` (Console/API billing) |
| `claude auth logout` | Log out | |
| `claude auth status` | Show auth status as JSON | `--text` (human-readable output) |

### `claude config`

| Sub-Command | Description | Example |
|-------------|-------------|---------|
| `claude config list` | Display all settings | |
| `claude config get <key>` | Retrieve specific setting | `claude config get model` |
| `claude config set <key> <value>` | Update setting | `claude config set model opus` |

### `claude mcp`

See [MCP Sub-Commands section](#11-mcp-sub-commands) below for full detail.

### `claude agents`

Lists all configured subagents, grouped by source.

### `claude update`

Updates to the latest version.

### `claude remote-control`

Starts a Remote Control server. Flags: `--name "session name"`.

---

## 4. System Prompt Flags

| Flag | Behavior | Example |
|------|----------|---------|
| `--system-prompt` | Replaces the entire default prompt | `claude --system-prompt "You are a Python expert"` |
| `--system-prompt-file` | Replaces with file contents | `claude --system-prompt-file ./prompts/review.txt` |
| `--append-system-prompt` | Appends to the default prompt | `claude --append-system-prompt "Always use TypeScript"` |
| `--append-system-prompt-file` | Appends file contents to default prompt | `claude --append-system-prompt-file ./style-rules.txt` |

- `--system-prompt` and `--system-prompt-file` are **mutually exclusive**.
- Append flags can be combined with either replacement flag.
- All four work in both interactive and non-interactive modes.

---

## 5. Permission Modes

| Mode | Description |
|------|-------------|
| `default` | Standard: prompts for permission on first use of each tool |
| `acceptEdits` | Automatically accepts file edit permissions for the session |
| `plan` | Plan Mode: Claude can analyze but not modify files or execute commands |
| `dontAsk` | Auto-denies tools unless pre-approved via `/permissions` or `permissions.allow` rules |
| `bypassPermissions` | Skips permission prompts except writes to `.git`, `.claude`, `.vscode`, `.idea` |

Set via: `--permission-mode <mode>` or `defaultMode` in settings.json.

---

## 6. Model Configuration

### Model Aliases

| Alias | Behavior |
|-------|----------|
| `default` | Recommended model based on account type |
| `sonnet` | Latest Sonnet (currently Sonnet 4.6) |
| `opus` | Latest Opus (currently Opus 4.6) |
| `haiku` | Fast and efficient Haiku model |
| `sonnet[1m]` | Sonnet with 1M token context window |
| `opus[1m]` | Opus with 1M token context window |
| `opusplan` | Opus for plan mode, Sonnet for execution |

### Setting Model (Priority Order)

1. During session: `/model <alias|name>`
2. At startup: `claude --model <alias|name>`
3. Environment variable: `ANTHROPIC_MODEL=<alias|name>`
4. Settings file: `"model": "opus"` in settings.json

### Effort Levels

| Level | Description |
|-------|-------------|
| `low` | Faster, cheaper for straightforward tasks |
| `medium` | Balanced (default for Opus 4.6 on Max/Team) |
| `high` | Deeper reasoning for complex problems |
| `max` | Deepest reasoning, no token constraint (Opus 4.6 only, session-scoped) |
| `auto` | Reset to model default |

Set via: `--effort`, `/effort`, `CLAUDE_CODE_EFFORT_LEVEL`, or `effortLevel` in settings.

---

## 7. Output Format Options

For `--output-format` (print mode only):

| Format | Description |
|--------|-------------|
| `text` | Plain text output (default) |
| `json` | Full JSON response after completion |
| `stream-json` | Streaming JSON events as they occur |

For `--input-format` (print mode only):

| Format | Description |
|--------|-------------|
| `text` | Plain text input (default) |
| `stream-json` | Streaming JSON input |

---

## 8. Print Mode (Non-Interactive) Reference

Print mode (`-p` / `--print`) is the key mode for wrapper/SDK usage. It runs a query and exits.

### Flags exclusive to or primarily used in print mode:

| Flag | Description |
|------|-------------|
| `--print` / `-p` | Enable non-interactive mode |
| `--output-format` | `text`, `json`, `stream-json` |
| `--input-format` | `text`, `stream-json` |
| `--include-partial-messages` | Include partial streaming events (requires `stream-json`) |
| `--json-schema` | Validated JSON output matching a JSON Schema |
| `--max-turns` | Limit agentic turns |
| `--max-budget-usd` | Maximum dollar spend |
| `--fallback-model` | Fallback model when primary is overloaded |
| `--no-session-persistence` | Don't save session to disk |
| `--permission-prompt-tool` | MCP tool for permission handling |

### Common print mode patterns:

```bash
# Basic query
claude -p "explain this function"

# With model selection
claude -p --model opus "complex analysis"

# With JSON output
claude -p --output-format json "query"

# With streaming JSON
claude -p --output-format stream-json "query"

# With structured output
claude -p --json-schema '{"type":"object","properties":{"answer":{"type":"string"}}}' "query"

# With budget limits
claude -p --max-turns 5 --max-budget-usd 2.00 "query"

# Piped input
cat file.py | claude -p "review this code"

# Continue previous session
claude -c -p "follow up question"

# With custom system prompt
claude -p --system-prompt "You are a code reviewer" "review this PR"

# With tool restrictions
claude -p --tools "Read" --allowedTools "Bash(git diff *)" "show me the diff"

# With fallback model
claude -p --model opus --fallback-model sonnet "query"

# With effort control
claude -p --effort high "complex analysis"
```

---

## 9. Environment Variables

### Authentication & API

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | Primary API key | None |
| `ANTHROPIC_AUTH_TOKEN` | Alternative bearer token (takes priority) | None |
| `ANTHROPIC_BASE_URL` | Custom API endpoint | api.anthropic.com |
| `ANTHROPIC_CUSTOM_HEADERS` | Custom HTTP headers (newline-separated) | None |
| `ANTHROPIC_BETAS` | Comma-separated beta feature headers | None |
| `ANTHROPIC_LOG` | SDK internal logging level | None |
| `API_TIMEOUT_MS` | Request timeout in milliseconds | 600000 |

### Model Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_MODEL` | Override default Claude model | None |
| `ANTHROPIC_SMALL_FAST_MODEL` | (Deprecated) Fast model for quick operations | Haiku |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | Default Haiku model ID | haiku-4.5 |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | Default Sonnet model ID | sonnet-4.6 |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | Default Opus model ID | opus-4.6 |
| `CLAUDE_CODE_SUBAGENT_MODEL` | Force model for sub-agents | None |
| `CLAUDE_CODE_MAX_OUTPUT_TOKENS` | Maximum output tokens | Per-model limit |
| `MAX_THINKING_TOKENS` | Extended thinking token budget | 0 (disabled) |
| `CLAUDE_CODE_EFFORT_LEVEL` | Reasoning effort: `low`, `medium`, `high`, `max`, `auto` | "high" |
| `ANTHROPIC_CUSTOM_MODEL_OPTION` | Custom entry for `/model` picker | None |
| `ANTHROPIC_CUSTOM_MODEL_OPTION_NAME` | Display name for custom model | Model ID |
| `ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION` | Description for custom model | Auto |

### AWS Bedrock Provider

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_USE_BEDROCK` | Route through AWS Bedrock |
| `CLAUDE_CODE_SKIP_BEDROCK_AUTH` | Skip AWS authentication |
| `BEDROCK_BASE_URL` | Custom Bedrock endpoint |
| `ANTHROPIC_BEDROCK_BASE_URL` | Alternative Bedrock endpoint |
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `AWS_SESSION_TOKEN` | AWS session token |
| `AWS_REGION` | AWS region |
| `AWS_DEFAULT_REGION` | Fallback AWS region |
| `AWS_PROFILE` | AWS credential profile |
| `ENABLE_PROMPT_CACHING_1H_BEDROCK` | 1-hour prompt caching on Bedrock |

### Google Vertex AI Provider

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_USE_VERTEX` | Route through Google Vertex AI |
| `CLAUDE_CODE_SKIP_VERTEX_AUTH` | Skip GCP authentication |
| `VERTEX_BASE_URL` | Custom Vertex AI endpoint |
| `ANTHROPIC_VERTEX_PROJECT_ID` | Google Cloud project ID |
| `CLOUD_ML_REGION` | GCP ML region |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to GCP credentials JSON |

### Microsoft Foundry Provider

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_USE_FOUNDRY` | Route through Azure Foundry |
| `CLAUDE_CODE_SKIP_FOUNDRY_AUTH` | Skip Azure authentication |
| `ANTHROPIC_FOUNDRY_API_KEY` | Foundry API key |
| `ANTHROPIC_FOUNDRY_BASE_URL` | Custom Foundry endpoint |
| `ANTHROPIC_FOUNDRY_RESOURCE` | Foundry resource name |

### OAuth & Login

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_OAUTH_TOKEN` | Pre-configured OAuth token |
| `CLAUDE_CODE_OAUTH_REFRESH_TOKEN` | OAuth refresh token |
| `CLAUDE_CODE_OAUTH_SCOPES` | Required OAuth scopes |
| `CLAUDE_CODE_OAUTH_CLIENT_ID` | Custom OAuth client ID |
| `CLAUDE_CODE_CUSTOM_OAUTH_URL` | Custom OAuth server URL |
| `CLAUDE_CODE_API_KEY_HELPER_TTL_MS` | Cache TTL for API key helper |
| `CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR` | File descriptor for API key |

### Core Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `CLAUDE_CONFIG_DIR` | Config files directory | ~/.claude |
| `CLAUDE_CODE_TMPDIR` | Temp directory for operations | /tmp |
| `CLAUDE_CODE_SHELL` | Shell for Bash tool execution | Auto-detected |
| `CLAUDE_CODE_SHELL_PREFIX` | Shell execution prefix command | None |
| `CLAUDE_CODE_EXTRA_BODY` | Additional JSON for API requests | None |
| `CLAUDE_CODE_BASE_REF` | Git base ref for diffs | Auto-detected |
| `CLAUDE_CODE_MAX_RETRIES` | Maximum API request retries | None |
| `CLAUDE_CODE_STREAMING_TEXT` | Enable streaming text mode | false |
| `CLAUDE_CODE_SYNTAX_HIGHLIGHT` | Enable syntax highlighting | true |
| `CLAUDE_CODE_ACCESSIBILITY` | Enable accessibility mode | false |

### Feature Enable Flags

| Variable | Description | Default |
|----------|-------------|---------|
| `ENABLE_LSP_TOOL` | Enable Language Server Protocol tool | false |
| `ENABLE_TOOL_SEARCH` | Tool search/deferred loading | "auto" |
| `ENABLE_SESSION_BACKGROUNDING` | Enable session backgrounding | false |
| `ENABLE_MCP_LARGE_OUTPUT_FILES` | Large output file support for MCP | false |
| `CLAUDE_CODE_ENABLE_TASKS` | Task list tools | true |
| `CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION` | Prompt suggestions | true |
| `CLAUDE_CODE_ENABLE_TELEMETRY` | Enable telemetry | true |
| `CLAUDE_CODE_ENABLE_CFC` | Chrome browser automation | false |

### Feature Disable Flags

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_DISABLE_1M_CONTEXT` | Disable 1M context models |
| `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING` | Disable adaptive thinking (revert to fixed budget) |
| `CLAUDE_CODE_DISABLE_ATTACHMENTS` | Disable file/image attachments |
| `CLAUDE_CODE_DISABLE_AUTO_MEMORY` | Disable automatic memory |
| `CLAUDE_CODE_DISABLE_CLAUDE_MDS` | Disable CLAUDE.md loading |
| `CLAUDE_CODE_DISABLE_COMMAND_INJECTION_CHECK` | Disable command injection check |
| `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS` | Disable experimental betas |
| `CLAUDE_CODE_DISABLE_FAST_MODE` | Disable fast/streaming mode |
| `CLAUDE_CODE_DISABLE_FILE_CHECKPOINTING` | Disable git-based checkpointing |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | Reduce non-critical requests |
| `CLAUDE_CODE_DISABLE_TERMINAL_TITLE` | Prevent terminal title updates |
| `CLAUDE_CODE_DISABLE_THINKING` | Completely disable thinking |
| `DISABLE_INTERLEAVED_THINKING` | Disable interleaved thinking |
| `DISABLE_PROMPT_CACHING` | Disable prompt caching (all models) |
| `DISABLE_PROMPT_CACHING_HAIKU` | Disable Haiku caching |
| `DISABLE_PROMPT_CACHING_SONNET` | Disable Sonnet caching |
| `DISABLE_PROMPT_CACHING_OPUS` | Disable Opus caching |
| `DISABLE_TELEMETRY` | Disable all telemetry |
| `DISABLE_AUTOUPDATER` | Disable auto-update checker |
| `DISABLE_COMPACT` | Disable all compaction |
| `DISABLE_AUTO_COMPACT` | Disable auto-compaction |

### Bash / Shell Tool

| Variable | Description | Default |
|----------|-------------|---------|
| `BASH_MAX_OUTPUT_LENGTH` | Max bash output characters | 30000 |
| `BASH_DEFAULT_TIMEOUT_MS` | Default bash command timeout (ms) | 120000 |
| `BASH_MAX_TIMEOUT_MS` | Max bash timeout (ms) | 600000 |

### MCP Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `MCP_TIMEOUT` | MCP server connection timeout (ms) | 30000 |
| `MCP_TOOL_TIMEOUT` | MCP tool execution timeout (ms) | Built-in |
| `MAX_MCP_OUTPUT_TOKENS` | Max MCP tool output tokens | 25000 |

### Tool Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` | Max concurrent tool executions | 10 |
| `CLAUDE_CODE_GLOB_TIMEOUT_SECONDS` | Glob operation timeout | 20 (60 on WSL) |
| `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS` | Max tokens for file read tool | Built-in |

### Proxy & TLS

| Variable | Description |
|----------|-------------|
| `HTTP_PROXY` | HTTP proxy URL |
| `HTTPS_PROXY` | HTTPS proxy URL |
| `NO_PROXY` | Domains to bypass proxy |
| `CLAUDE_CODE_CLIENT_CERT` | Client TLS certificate path |
| `CLAUDE_CODE_CLIENT_KEY` | Client TLS key path |
| `CLAUDE_CODE_CLIENT_KEY_PASSPHRASE` | Client key passphrase |
| `NODE_EXTRA_CA_CERTS` | Additional CA certificates |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Reject unauthorized TLS ("1" = yes) |

### Context & Compaction

| Variable | Description |
|----------|-------------|
| `DISABLE_COMPACT` | Disable all compaction |
| `DISABLE_AUTO_COMPACT` | Disable auto-compaction |
| `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` | Auto-compact trigger percentage |

### Agent Teams

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | Enable agent teams |
| `CLAUDE_CODE_PLAN_MODE_REQUIRED` | Require plan mode |

### Observability (OpenTelemetry)

| Variable | Description |
|----------|-------------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP exporter endpoint |
| `OTEL_EXPORTER_OTLP_HEADERS` | OTLP exporter headers |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | OTLP protocol |
| `OTEL_LOG_USER_PROMPTS` | Include user prompts in logs |
| `OTEL_LOG_TOOL_CONTENT` | Include tool content in logs |
| `OTEL_LOG_TOOL_DETAILS` | Include tool details in logs |

### CI/CD Detection

| Variable | Description |
|----------|-------------|
| `GITHUB_ACTIONS` | GitHub Actions indicator |
| `GITLAB_CI` | GitLab CI indicator |
| `CIRCLECI` | CircleCI indicator |
| `BUILDKITE` | Buildkite indicator |
| `CODESPACES` | GitHub Codespaces indicator |

### Remote / Cowork

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_REMOTE` | Running in remote/server mode |
| `CLAUDE_CODE_REMOTE_SESSION_ID` | Remote session identifier |

---

## 10. Settings.json Keys

The `settings.json` file supports these keys (at user `~/.claude/settings.json`, project `.claude/settings.json`, or local `.claude/settings.local.json`):

### Core Settings

| Key | Description | Example |
|-----|-------------|---------|
| `model` | Override default model | `"claude-sonnet-4-6"` |
| `availableModels` | Restrict selectable models | `["sonnet", "haiku"]` |
| `modelOverrides` | Map Anthropic model IDs to provider-specific IDs | `{"claude-opus-4-6": "arn:aws:..."}` |
| `effortLevel` | Persist effort level | `"medium"` |
| `language` | Preferred response language | `"japanese"` |
| `outputStyle` | Output style adjustment | `"Explanatory"` |
| `agent` | Run main thread as named subagent | `"code-reviewer"` |

### Permissions

| Key | Description |
|-----|-------------|
| `permissions.allow` | Array of permission rules to allow |
| `permissions.ask` | Array of permission rules to ask confirmation |
| `permissions.deny` | Array of permission rules to deny |
| `permissions.additionalDirectories` | Additional working directories |
| `permissions.defaultMode` | Default permission mode |
| `permissions.disableBypassPermissionsMode` | Set `"disable"` to prevent bypass mode |

### Authentication

| Key | Description |
|-----|-------------|
| `apiKeyHelper` | Script to generate auth value |
| `forceLoginMethod` | Restrict login: `"claudeai"` or `"console"` |
| `forceLoginOrgUUID` | Auto-select organization UUID |
| `awsAuthRefresh` | Custom AWS auth refresh script |
| `awsCredentialExport` | Custom AWS credential export script |

### Hooks & Automation

| Key | Description |
|-----|-------------|
| `hooks` | Custom commands at lifecycle events |
| `disableAllHooks` | Disable all hooks |
| `allowManagedHooksOnly` | (Managed only) Only managed/SDK hooks |
| `allowedHttpHookUrls` | Allowlist of URL patterns for HTTP hooks |
| `httpHookAllowedEnvVars` | Allowlist of env vars for HTTP hooks |

### MCP Configuration

| Key | Description |
|-----|-------------|
| `enableAllProjectMcpServers` | Auto-approve all project MCP servers |
| `enabledMcpjsonServers` | Specific MCP servers to approve |
| `disabledMcpjsonServers` | Specific MCP servers to reject |
| `allowManagedMcpServersOnly` | (Managed only) Only managed MCP servers |
| `allowedMcpServers` | (Managed) Allowlist of MCP servers |
| `deniedMcpServers` | (Managed) Denylist of MCP servers |

### Sandbox

| Key | Description |
|-----|-------------|
| `sandbox.enabled` | Enable bash sandboxing |
| `sandbox.autoAllowBashIfSandboxed` | Auto-approve bash when sandboxed |
| `sandbox.excludedCommands` | Commands that run outside sandbox |
| `sandbox.filesystem.allowWrite` | Additional writable paths |
| `sandbox.filesystem.denyWrite` | Denied write paths |
| `sandbox.filesystem.denyRead` | Denied read paths |
| `sandbox.filesystem.allowRead` | Re-allowed read paths |
| `sandbox.network.allowedDomains` | Allowed outbound domains |
| `sandbox.network.allowUnixSockets` | Allowed Unix socket paths |
| `sandbox.network.allowLocalBinding` | Allow localhost port binding |

### UI & Display

| Key | Description |
|-----|-------------|
| `cleanupPeriodDays` | Session retention period (default: 30) |
| `autoUpdatesChannel` | Release channel: `"stable"` or `"latest"` |
| `alwaysThinkingEnabled` | Extended thinking on by default |
| `voiceEnabled` | Push-to-talk voice dictation |
| `spinnerVerbs` | Custom spinner action verbs |
| `spinnerTipsEnabled` | Show tips in spinner |
| `spinnerTipsOverride` | Override spinner tips |
| `prefersReducedMotion` | Reduce UI animations |
| `teammateMode` | Agent team display: `auto`, `in-process`, `tmux` |
| `statusLine` | Custom status line config |
| `fileSuggestion` | Custom `@` file autocomplete |
| `respectGitignore` | File picker respects .gitignore |
| `plansDirectory` | Where plan files are stored |
| `fastModePerSessionOptIn` | Require per-session fast mode opt-in |
| `feedbackSurveyRate` | Survey appearance probability (0-1) |

### Attribution

| Key | Description |
|-----|-------------|
| `attribution.commit` | Git commit attribution text |
| `attribution.pr` | PR description attribution text |
| `includeCoAuthoredBy` | (Deprecated) Co-authored-by in commits |
| `includeGitInstructions` | Include git workflow instructions |

### Worktree

| Key | Description |
|-----|-------------|
| `worktree.symlinkDirectories` | Directories to symlink in worktrees |
| `worktree.sparsePaths` | Paths for git sparse-checkout |

### Announcements

| Key | Description |
|-----|-------------|
| `companyAnnouncements` | Messages displayed at startup |

### Environment

| Key | Description |
|-----|-------------|
| `env` | Environment variables for every session |

---

## 11. MCP Sub-Commands

### Adding Servers

```bash
# Add HTTP transport server
claude mcp add --transport http <name> <url>
claude mcp add --transport http notion https://mcp.notion.com/mcp

# Add SSE transport server
claude mcp add --transport sse <name> <url>
claude mcp add --transport sse asana https://mcp.asana.com/sse

# Add stdio transport server (-- separates claude flags from server command)
claude mcp add --transport stdio <name> -- <command> [args...]
claude mcp add --transport stdio --env AIRTABLE_API_KEY=YOUR_KEY airtable -- npx -y airtable-mcp-server

# Add from JSON definition
claude mcp add-json <name> '<json>'
claude mcp add-json weather-api '{"type":"http","url":"https://api.weather.com/mcp"}'

# Import from Claude Desktop
claude mcp add-from-claude-desktop
```

### `claude mcp add` Flags

| Flag | Description |
|------|-------------|
| `--transport <type>` | Transport type: `http`, `sse`, `stdio` |
| `--scope <scope>` | Scope: `local` (default), `project`, `user` |
| `--env <KEY=VALUE>` | Environment variable for the server (repeatable) |
| `--header <header>` | HTTP header for remote servers |
| `--client-id <id>` | OAuth client ID |
| `--client-secret` | Prompt for OAuth client secret (masked input) |
| `--callback-port <port>` | OAuth callback port |

### Managing Servers

```bash
# List all configured servers
claude mcp list

# Get details for a specific server
claude mcp get <name>

# Remove a server
claude mcp remove <name>

# Reset project-scoped approval choices
claude mcp reset-project-choices

# Run Claude Code as an MCP server itself
claude mcp serve
```

---

## 12. Slash Commands (In-Session)

### Project Management

| Command | Purpose |
|---------|---------|
| `/init` | Auto-generate CLAUDE.md |
| `/memory` | Edit memory file |
| `/context` | Visualize context window usage |
| `/compact` | Compress/summarize context |
| `/clear` | Reset conversation history |
| `/resume` | Resume past session |
| `/fork` | Branch conversation into new session |
| `/rename` | Rename session |
| `/add-dir <path>` | Add directory to context |
| `/copy` | Select and copy code blocks |

### Information & Status

| Command | Purpose |
|---------|---------|
| `/usage` | Check token usage vs plan limits |
| `/cost` | Show session cost breakdown |
| `/help` | List available commands |
| `/tasks` | Check background tasks |
| `/doctor` | Run environment diagnostics |
| `/stats` | Generate usage HTML report |
| `/debug` | Show troubleshooting info |
| `/effort <level>` | Switch effort (low/medium/high/max) |
| `/extra-usage` | Enable additional capacity |

### Mode & Model Control

| Command | Purpose |
|---------|---------|
| `/model` | Switch model version |
| `/fast` | Toggle Fast Mode |
| `/plan [description]` | Toggle Plan Mode |
| `/vim` | Toggle Vim-style editing |
| `/output-style` | Change output presentation |

### Feature Management

| Command | Purpose |
|---------|---------|
| `/hooks` | Configure and manage hooks |
| `/agents` | Create and manage sub-agents |
| `/permissions` | Change permission settings |
| `/sandbox` | Enable sandbox execution mode |
| `/config` | Open settings interface |
| `/login` | Re-authenticate |
| `/rewind` | Rewind conversation/code changes |

### Integration & Extensions

| Command | Purpose |
|---------|---------|
| `/install-github-app` | Setup GitHub PR auto-review |
| `/plugin` | Plugin management |
| `/mcp` | Check MCP status/OAuth auth |
| `/rc` | Switch to Remote Control |
| `/review <PR#>` | Code review for PR |
| `/pr-comments` | Show PR comments |
| `/security-review` | Security audit of changes |
| `/skills` | Skill management menu |

### Advanced (v2.1.63+)

| Command | Purpose |
|---------|---------|
| `/simplify` | 3-agent review pipeline |
| `/batch` | Large-scale parallel changes |
| `/bug` | Report issues to Anthropic |
| `/release-notes` | View version changelog |

### Terminal Settings

| Command | Purpose |
|---------|---------|
| `/terminal-setup` | Set Shift+Enter keybinding |
| `/keybindings` | Open keybindings config |
| `/status-line` | Setup terminal status line |
| `/voice` | Toggle voice dictation |

---

## 13. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Esc` | Stop processing |
| `Esc` x 2 | Rewind menu (selective rollback) |
| `Enter` | Send message |
| `Shift+Enter` | Insert newline |
| `\` + `Enter` | Newline (all terminals) |
| `Up` | Browse command history |
| `Ctrl+C` | Exit session (press twice) |
| `Ctrl+F` | Stop all background agents |
| `Ctrl+R` | Reverse search history |
| `Ctrl+Y` | Readline-style paste |
| `Ctrl+S` | Stash current prompt |
| `Ctrl+T` | Toggle task list display |
| `Ctrl+O` | Toggle verbose display |
| `Ctrl+G` | Open plan in external editor |
| `Shift+Tab` | Cycle modes (normal -> auto-accept -> plan) |
| `Tab` | Toggle extended thinking on/off |
| `Shift+Down` | Navigate between Agent Team members |
| `Shift+Drag` | Add file as reference |
| `Ctrl+V` | Paste image from clipboard |
| `?` | Show available shortcuts |

---

## 14. Prompt Notation

| Notation | Purpose | Example |
|----------|---------|---------|
| `@<filename>` | Reference file/directory | `@./src/main.rs` |
| `@terminal:<name>` | Reference terminal output | `@terminal:build` |
| `#<content>` | Add to CLAUDE.md memory | `#always use TypeScript` |
| `!<command>` | Execute shell command inline | `!npm test` |
| `& <task>` | Run background task | `& analyze the codebase` |

---

## Sources

- [Official CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Settings Documentation](https://code.claude.com/docs/en/settings)
- [Permissions Documentation](https://code.claude.com/docs/en/permissions)
- [MCP Documentation](https://code.claude.com/docs/en/mcp)
- [Model Configuration](https://code.claude.com/docs/en/model-config)
- [SmartScope Complete Reference (2026 Edition)](https://smartscope.blog/en/generative-ai/claude/claude-code-reference-guide/)
- [Introl Blog Technical Reference](https://introl.com/blog/claude-code-cli-comprehensive-guide-2025)
- [GitHub: Claude Code Environment Variables Gist](https://gist.github.com/unkn0wncode/f87295d055dd0f0e8082358a0b5cc467)
- [GitHub: anthropics/claude-code](https://github.com/anthropics/claude-code)
- [Shipyard Cheatsheet](https://shipyard.build/blog/claude-code-cheat-sheet/)
