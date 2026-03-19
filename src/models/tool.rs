use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ToolMetadataResponse {
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: HashMap<String, String>,
    pub examples: Vec<String>,
    pub is_safe: bool,
    pub requires_network: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolMetadataResponse>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolConfigurationResponse {
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub effective_tools: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolConfigurationRequest {
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolValidationResponse {
    pub valid: HashMap<String, bool>,
    pub invalid_tools: Vec<String>,
}
