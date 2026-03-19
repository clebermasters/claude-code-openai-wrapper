use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::constants::{CLAUDE_TOOLS, DEFAULT_ALLOWED_TOOLS, DEFAULT_DISALLOWED_TOOLS};
use crate::models::tool::ToolMetadataResponse;

#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: HashMap<String, String>,
    pub examples: Vec<String>,
    pub is_safe: bool,
    pub requires_network: bool,
}

impl ToolMetadata {
    pub fn to_response(&self) -> ToolMetadataResponse {
        ToolMetadataResponse {
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            parameters: self.parameters.clone(),
            examples: self.examples.clone(),
            is_safe: self.is_safe,
            requires_network: self.requires_network,
        }
    }
}

fn build_tool_metadata() -> HashMap<String, ToolMetadata> {
    let mut m = HashMap::new();

    let tools = vec![
        ("Task", "Launch specialized agents for complex, multi-step tasks", "agent", false, false,
         vec![("description", "Short description of the task"), ("prompt", "Detailed task instructions")],
         vec!["Launch a general-purpose agent to refactor code"]),
        ("Bash", "Execute bash commands in a persistent shell session", "system", true, false,
         vec![("command", "The bash command to execute"), ("timeout", "Optional timeout in milliseconds")],
         vec!["Run npm install", "Execute git status"]),
        ("Glob", "Fast file pattern matching with glob patterns", "file", true, false,
         vec![("pattern", "Glob pattern to match files"), ("path", "Directory to search in")],
         vec!["Find all Python files: **/*.py"]),
        ("Grep", "Search file contents using regex patterns", "file", true, false,
         vec![("pattern", "Regex pattern to search for"), ("path", "File or directory to search in")],
         vec!["Search for function definitions"]),
        ("Read", "Read files from the local filesystem", "file", true, false,
         vec![("file_path", "Absolute path to the file"), ("offset", "Line number to start reading from")],
         vec!["Read entire file"]),
        ("Edit", "Perform exact string replacements in files", "file", true, false,
         vec![("file_path", "Absolute path to file"), ("old_string", "Text to replace"), ("new_string", "Replacement text")],
         vec!["Fix a bug by replacing code"]),
        ("Write", "Write or overwrite files on the filesystem", "file", true, false,
         vec![("file_path", "Absolute path to file"), ("content", "Content to write")],
         vec!["Create a new file"]),
        ("NotebookEdit", "Edit Jupyter notebook cells", "file", true, false,
         vec![("notebook_path", "Path to .ipynb file"), ("cell_id", "ID of cell to edit")],
         vec!["Replace code in notebook cell"]),
        ("WebFetch", "Fetch and process web content", "web", true, true,
         vec![("url", "URL to fetch content from")],
         vec!["Fetch documentation page"]),
        ("TodoWrite", "Create and manage task lists", "productivity", true, false,
         vec![("todos", "Array of todo items")],
         vec!["Create task list for feature"]),
        ("WebSearch", "Search the web for current information", "web", true, true,
         vec![("query", "Search query")],
         vec!["Search for latest documentation"]),
        ("BashOutput", "Retrieve output from background bash shells", "system", true, false,
         vec![("bash_id", "ID of the background shell")],
         vec!["Check output of running process"]),
        ("KillShell", "Kill a running background bash shell", "system", true, false,
         vec![("shell_id", "ID of the shell to kill")],
         vec!["Stop long-running background process"]),
        ("Skill", "Execute specialized skills", "productivity", true, false,
         vec![("command", "Skill name to execute")],
         vec!["Execute PDF processing skill"]),
        ("SlashCommand", "Execute custom slash commands", "productivity", true, false,
         vec![("command", "Slash command with arguments")],
         vec!["Run custom code review command"]),
    ];

    for (name, desc, cat, is_safe, net, params, examples) in tools {
        m.insert(
            name.to_string(),
            ToolMetadata {
                name: name.to_string(),
                description: desc.to_string(),
                category: cat.to_string(),
                parameters: params.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                examples: examples.into_iter().map(|e| e.to_string()).collect(),
                is_safe,
                requires_network: net,
            },
        );
    }
    m
}

#[derive(Debug, Clone)]
pub struct ToolConfiguration {
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ToolConfiguration {
    pub fn get_effective_tools(&self) -> HashSet<String> {
        let mut effective: HashSet<String> = if let Some(allowed) = &self.allowed_tools {
            allowed.iter().cloned().collect()
        } else {
            CLAUDE_TOOLS.iter().map(|s| s.to_string()).collect()
        };

        if let Some(disallowed) = &self.disallowed_tools {
            for tool in disallowed {
                effective.remove(tool);
            }
        }

        effective
    }

    pub fn update(&mut self, allowed: Option<Vec<String>>, disallowed: Option<Vec<String>>) {
        if let Some(a) = allowed {
            self.allowed_tools = Some(a);
        }
        if let Some(d) = disallowed {
            self.disallowed_tools = Some(d);
        }
        self.updated_at = Utc::now();
    }
}

impl Default for ToolConfiguration {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            allowed_tools: None,
            disallowed_tools: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone)]
pub struct ToolManager {
    metadata: Arc<HashMap<String, ToolMetadata>>,
    global_config: Arc<RwLock<ToolConfiguration>>,
    session_configs: Arc<RwLock<HashMap<String, ToolConfiguration>>>,
}

impl ToolManager {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            metadata: Arc::new(build_tool_metadata()),
            global_config: Arc::new(RwLock::new(ToolConfiguration {
                allowed_tools: Some(DEFAULT_ALLOWED_TOOLS.iter().map(|s| s.to_string()).collect()),
                disallowed_tools: Some(DEFAULT_DISALLOWED_TOOLS.iter().map(|s| s.to_string()).collect()),
                created_at: now,
                updated_at: now,
            })),
            session_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn list_all_tools(&self) -> Vec<ToolMetadataResponse> {
        self.metadata.values().map(|m| m.to_response()).collect()
    }

    pub async fn get_global_config(&self) -> ToolConfiguration {
        self.global_config.read().await.clone()
    }

    pub async fn update_global_config(&self, allowed: Option<Vec<String>>, disallowed: Option<Vec<String>>) -> ToolConfiguration {
        let mut config = self.global_config.write().await;
        config.update(allowed, disallowed);
        info!("Updated global tool config");
        config.clone()
    }

    pub async fn get_effective_config(&self, session_id: Option<&str>) -> ToolConfiguration {
        if let Some(sid) = session_id {
            let configs = self.session_configs.read().await;
            if let Some(config) = configs.get(sid) {
                return config.clone();
            }
        }
        self.global_config.read().await.clone()
    }

    pub async fn set_session_config(&self, session_id: &str, allowed: Option<Vec<String>>, disallowed: Option<Vec<String>>) -> ToolConfiguration {
        let mut configs = self.session_configs.write().await;
        let config = configs.entry(session_id.to_string()).or_default();
        config.update(allowed, disallowed);
        info!("Updated session {session_id} tool config");
        config.clone()
    }

    pub async fn get_effective_tools(&self, session_id: Option<&str>) -> Vec<String> {
        let config = self.get_effective_config(session_id).await;
        let mut tools: Vec<String> = config.get_effective_tools().into_iter().collect();
        tools.sort();
        tools
    }

    pub fn validate_tools(&self, names: &[String]) -> HashMap<String, bool> {
        names.iter().map(|n| (n.clone(), CLAUDE_TOOLS.contains(&n.as_str()))).collect()
    }

    #[cfg(test)]
    pub async fn delete_session_config(&self, session_id: &str) -> bool {
        self.session_configs.write().await.remove(session_id).is_some()
    }

    pub async fn get_stats(&self) -> serde_json::Value {
        let config = self.global_config.read().await;
        let session_count = self.session_configs.read().await.len();

        serde_json::json!({
            "total_tools": CLAUDE_TOOLS.len(),
            "global_allowed": config.allowed_tools.as_ref().map_or(0, |a| a.len()),
            "global_disallowed": config.disallowed_tools.as_ref().map_or(0, |d| d.len()),
            "session_configs": session_count,
            "tool_categories": {
                "file": self.metadata.values().filter(|t| t.category == "file").count(),
                "system": self.metadata.values().filter(|t| t.category == "system").count(),
                "web": self.metadata.values().filter(|t| t.category == "web").count(),
                "productivity": self.metadata.values().filter(|t| t.category == "productivity").count(),
                "agent": self.metadata.values().filter(|t| t.category == "agent").count(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ToolConfiguration ---

    #[test]
    fn test_effective_tools_default() {
        let config = ToolConfiguration::default();
        let tools = config.get_effective_tools();
        // No allowed/disallowed => all CLAUDE_TOOLS
        assert_eq!(tools.len(), CLAUDE_TOOLS.len());
    }

    #[test]
    fn test_effective_tools_with_allowed() {
        let config = ToolConfiguration {
            allowed_tools: Some(vec!["Read".into(), "Write".into()]),
            disallowed_tools: None,
            ..Default::default()
        };
        let tools = config.get_effective_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains("Read"));
        assert!(tools.contains("Write"));
    }

    #[test]
    fn test_effective_tools_with_disallowed() {
        let config = ToolConfiguration {
            allowed_tools: None,
            disallowed_tools: Some(vec!["Task".into(), "WebFetch".into()]),
            ..Default::default()
        };
        let tools = config.get_effective_tools();
        assert!(!tools.contains("Task"));
        assert!(!tools.contains("WebFetch"));
        assert!(tools.contains("Read"));
    }

    #[test]
    fn test_effective_tools_both() {
        let config = ToolConfiguration {
            allowed_tools: Some(vec!["Read".into(), "Write".into(), "Bash".into()]),
            disallowed_tools: Some(vec!["Bash".into()]),
            ..Default::default()
        };
        let tools = config.get_effective_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains("Read"));
        assert!(!tools.contains("Bash"));
    }

    #[test]
    fn test_config_update() {
        let mut config = ToolConfiguration::default();
        let before = config.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        config.update(Some(vec!["Read".into()]), None);
        assert_eq!(config.allowed_tools, Some(vec!["Read".into()]));
        assert!(config.updated_at >= before);
    }

    // --- ToolManager ---

    #[tokio::test]
    async fn test_list_all_tools() {
        let mgr = ToolManager::new();
        let tools = mgr.list_all_tools();
        assert_eq!(tools.len(), CLAUDE_TOOLS.len());
    }

    #[test]
    fn test_validate_tools() {
        let mgr = ToolManager::new();
        let result = mgr.validate_tools(&["Read".into(), "Fake".into()]);
        assert_eq!(result["Read"], true);
        assert_eq!(result["Fake"], false);
    }

    #[tokio::test]
    async fn test_get_effective_tools_default() {
        let mgr = ToolManager::new();
        let tools = mgr.get_effective_tools(None).await;
        // Global config has DEFAULT_ALLOWED_TOOLS minus DEFAULT_DISALLOWED_TOOLS
        assert!(tools.contains(&"Read".to_string()));
        assert!(!tools.contains(&"Task".to_string()));
    }

    #[tokio::test]
    async fn test_session_config_overrides_global() {
        let mgr = ToolManager::new();
        mgr.set_session_config("s1", Some(vec!["Bash".into()]), None).await;
        let tools = mgr.get_effective_tools(Some("s1")).await;
        assert_eq!(tools, vec!["Bash".to_string()]);
    }

    #[tokio::test]
    async fn test_update_global_config() {
        let mgr = ToolManager::new();
        mgr.update_global_config(Some(vec!["Read".into(), "Grep".into()]), None).await;
        let tools = mgr.get_effective_tools(None).await;
        assert!(tools.contains(&"Read".to_string()));
        assert!(tools.contains(&"Grep".to_string()));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let mgr = ToolManager::new();
        mgr.set_session_config("s1", Some(vec!["Read".into()]), None).await;
        let stats = mgr.get_stats().await;
        assert_eq!(stats["total_tools"], CLAUDE_TOOLS.len());
        assert_eq!(stats["session_configs"], 1);
    }
}
