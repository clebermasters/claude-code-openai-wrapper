use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use crate::config::Config;

/// Claude CLI subprocess integration.
/// Replaces the Python claude-agent-sdk with direct CLI calls.
#[derive(Clone)]
pub struct ClaudeCli {
    config: Arc<Config>,
    cwd: PathBuf,
    auth_env_vars: HashMap<String, String>,
    cli_path: String,
}

impl ClaudeCli {
    pub fn new(config: Arc<Config>, auth_env_vars: HashMap<String, String>) -> Result<Self, String> {
        let cwd = if let Some(ref cwd_str) = config.claude_cwd {
            let path = PathBuf::from(cwd_str);
            if !path.exists() {
                return Err(format!("Working directory does not exist: {}", cwd_str));
            }
            info!("Using CLAUDE_CWD: {}", cwd_str);
            path
        } else {
            let tmp = std::env::temp_dir().join(format!("claude_code_workspace_{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&tmp)
                .map_err(|e| format!("Failed to create temp workspace: {e}"))?;
            info!("Using temporary isolated workspace: {}", tmp.display());
            tmp
        };

        Ok(Self {
            cli_path: config.claude_cli_path.clone(),
            config,
            cwd,
            auth_env_vars,
        })
    }

    /// Verify Claude CLI is accessible and working.
    pub async fn verify_cli(&self) -> bool {
        info!("Testing Claude CLI...");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.run_command(&["--version"]),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                info!("Claude CLI verified: {}", output.trim());
                true
            }
            Ok(Err(e)) => {
                error!("Claude CLI verification failed: {e}");
                warn!("Please ensure Claude Code is installed: npm install -g @anthropic-ai/claude-code");
                false
            }
            Err(_) => {
                warn!("Claude CLI verification timed out (30s)");
                false
            }
        }
    }

    /// Run a simple CLI command and return stdout.
    async fn run_command(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.cli_path)
            .args(args)
            .current_dir(&self.cwd)
            .envs(&self.auth_env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Claude CLI exited with error: {stderr}"))
        }
    }

    /// Build the full CLI arguments for a completion.
    fn build_args(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        model: Option<&str>,
        allowed_tools: Option<&[String]>,
        disallowed_tools: Option<&[String]>,
        permission_mode: Option<&str>,
        stream_json: bool,
    ) -> Vec<String> {
        let mut args = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            if stream_json { "stream-json".to_string() } else { "json".to_string() },
        ];

        // stream-json requires --verbose
        if stream_json {
            args.push("--verbose".to_string());
        }

        if let Some(m) = model {
            args.push("--model".to_string());
            args.push(m.to_string());
        }

        if let Some(sp) = system_prompt {
            args.push("--system-prompt".to_string());
            args.push(sp.to_string());
        }

        // Handle tool configuration
        if let Some(tools) = allowed_tools {
            if !tools.is_empty() {
                args.push("--allowedTools".to_string());
                args.push(tools.join(","));
            }
        }

        if let Some(tools) = disallowed_tools {
            if !tools.is_empty() {
                // Check if ALL tools are being disabled
                let all_tools: std::collections::HashSet<&str> =
                    crate::constants::CLAUDE_TOOLS.iter().copied().collect();
                let disallowed_set: std::collections::HashSet<&str> =
                    tools.iter().map(|s| s.as_str()).collect();
                if disallowed_set == all_tools {
                    // Don't pass any tools flags - CLI in --print mode runs
                    // without tools by default. Passing --tools "" breaks arg parsing.
                } else {
                    args.push("--disallowedTools".to_string());
                    args.push(tools.join(","));
                }
            }
        }

        if let Some(pm) = permission_mode {
            args.push("--permission-mode".to_string());
            args.push(pm.to_string());
        }

        // Prompt goes last as a positional argument
        args.push(prompt.to_string());

        args
    }

    /// Non-streaming completion: run CLI and return the full response text.
    pub async fn run_completion(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        model: Option<&str>,
        allowed_tools: Option<&[String]>,
        disallowed_tools: Option<&[String]>,
        permission_mode: Option<&str>,
    ) -> Result<CompletionResult, String> {
        let args = self.build_args(
            prompt, system_prompt, model,
            allowed_tools, disallowed_tools, permission_mode, false,
        );

        debug!("Running claude CLI with args: {:?}", args);

        let timeout = std::time::Duration::from_millis(self.config.max_timeout_ms);

        let result = tokio::time::timeout(timeout, async {
            let output = Command::new(&self.cli_path)
                .args(&args)
                .current_dir(&self.cwd)
                .envs(&self.auth_env_vars)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if !stderr.is_empty() {
                debug!("Claude CLI stderr: {stderr}");
            }

            if !output.status.success() && stdout.is_empty() {
                return Err(format!("Claude CLI error: {stderr}"));
            }

            Ok(stdout)
        })
        .await
        .map_err(|_| "Claude CLI timed out".to_string())?;

        let stdout = result?;

        // Parse the JSON output
        let text = self.extract_result_text(&stdout);
        let metadata = self.extract_metadata_from_output(&stdout);

        Ok(CompletionResult {
            text: text.unwrap_or_default(),
            metadata,
        })
    }

    /// Streaming completion: run CLI with stream-json and return lines via a channel.
    pub async fn run_completion_stream(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        model: Option<&str>,
        allowed_tools: Option<&[String]>,
        disallowed_tools: Option<&[String]>,
        permission_mode: Option<&str>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, String> {
        let args = self.build_args(
            prompt, system_prompt, model,
            allowed_tools, disallowed_tools, permission_mode, true,
        );

        debug!("Running claude CLI (streaming) with args: {:?}", args);

        let mut child = Command::new(&self.cli_path)
            .args(&args)
            .current_dir(&self.cwd)
            .envs(&self.auth_env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn claude CLI: {e}"))?;

        let stdout = child.stdout.take().ok_or("No stdout from CLI")?;
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        let timeout_ms = self.config.max_timeout_ms;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let deadline = tokio::time::Instant::now() + timeout;

            loop {
                let read_result = tokio::time::timeout_at(deadline, lines.next_line()).await;

                match read_result {
                    Ok(Ok(Some(line))) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        debug!("CLI stream line: {}", &line[..line.len().min(200)]);

                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(json) => {
                                let event = parse_stream_event(&json);
                                if tx.send(event).await.is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            Err(e) => {
                                debug!("Non-JSON line from CLI: {e}");
                                // Send as raw text
                                if tx.send(StreamEvent::Text(line)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => break, // EOF
                    Ok(Err(e)) => {
                        error!("Error reading CLI stdout: {e}");
                        let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx.send(StreamEvent::Error("Claude CLI timed out".to_string())).await;
                        break;
                    }
                }
            }

            let _ = tx.send(StreamEvent::Done).await;
            let _ = child.wait().await;
        });

        Ok(rx)
    }

    /// Extract the result text from JSON output.
    fn extract_result_text(&self, output: &str) -> Option<String> {
        // Try to parse as a single JSON object first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            // Check for "result" field (ResultMessage format)
            if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                return Some(result.to_string());
            }
            // Check for content array
            if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
                let texts: Vec<&str> = content
                    .iter()
                    .filter_map(|block| {
                        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                            block.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }

        // Try parsing as NDJSON (multiple JSON objects per line)
        let mut last_text = None;
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                // ResultMessage with result
                if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        return Some(result.to_string());
                    }
                }
                // AssistantMessage: {"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}}
                if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                    if let Some(msg) = json.get("message") {
                        if let Some(text) = extract_text_from_content(msg.get("content")) {
                            last_text = Some(text);
                        }
                    }
                }
                // Direct content array
                if let Some(text) = extract_text_from_content(json.get("content")) {
                    last_text = Some(text);
                }
            }
        }

        last_text
    }

    fn extract_metadata_from_output(&self, output: &str) -> CompletionMetadata {
        let mut metadata = CompletionMetadata::default();

        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
                    metadata.total_cost_usd = json.get("total_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    metadata.duration_ms = json.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    metadata.num_turns = json.get("num_turns").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    metadata.session_id = json.get("session_id").and_then(|v| v.as_str()).map(String::from);
                }
                if json.get("subtype").and_then(|s| s.as_str()) == Some("init") {
                    if let Some(data) = json.get("data") {
                        metadata.session_id = data.get("session_id").and_then(|v| v.as_str()).map(String::from);
                        metadata.model = data.get("model").and_then(|v| v.as_str()).map(String::from);
                    }
                }
            }
        }

        metadata
    }

    /// Estimate token usage based on character count (~4 chars/token).
    pub fn estimate_token_usage(&self, prompt: &str, completion: &str) -> (u32, u32) {
        let prompt_tokens = (prompt.len() / 4).max(1) as u32;
        let completion_tokens = (completion.len() / 4).max(1) as u32;
        (prompt_tokens, completion_tokens)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompletionMetadata {
    pub session_id: Option<String>,
    pub total_cost_usd: f64,
    pub duration_ms: u64,
    pub num_turns: u32,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub text: String,
    pub metadata: CompletionMetadata,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Text(String),
    AssistantText(String),
    Result(String),
    Error(String),
    Done,
}

/// Extract text from a content array (handles [{"type":"text","text":"..."},...])
fn extract_text_from_content(content: Option<&serde_json::Value>) -> Option<String> {
    let arr = content?.as_array()?;
    let texts: Vec<&str> = arr
        .iter()
        .filter_map(|block| {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                block.get("text").and_then(|t| t.as_str())
            } else {
                None
            }
        })
        .collect();
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn parse_stream_event(json: &serde_json::Value) -> StreamEvent {
    // ResultMessage with result text
    if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
        if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
            return StreamEvent::Result(result.to_string());
        }
    }

    // AssistantMessage: {"type":"assistant","message":{"content":[...]}}
    if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
        if let Some(msg) = json.get("message") {
            if let Some(text) = extract_text_from_content(msg.get("content")) {
                return StreamEvent::AssistantText(text);
            }
        }
    }

    // Direct content array fallback
    if let Some(text) = extract_text_from_content(json.get("content")) {
        return StreamEvent::AssistantText(text);
    }

    // Error in result
    if json.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
        let msg = json.get("result")
            .or_else(|| json.get("error_message"))
            .or_else(|| json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return StreamEvent::Error(msg.to_string());
    }

    // Default: skip system/rate_limit messages
    StreamEvent::Text(String::new())
}
