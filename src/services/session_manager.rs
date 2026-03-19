use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::constants::SESSION_CLEANUP_INTERVAL_SECS;
use crate::models::openai::Message;
use crate::models::session::{SessionInfo, SessionListResponse};

#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn new(session_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            messages: Vec::new(),
            created_at: now,
            last_accessed: now,
            expires_at: now + Duration::hours(1),
        }
    }

    pub fn touch(&mut self) {
        let now = Utc::now();
        self.last_accessed = now;
        self.expires_at = now + Duration::hours(1);
    }

    pub fn add_messages(&mut self, messages: &[Message]) {
        self.messages.extend(messages.iter().cloned());
        self.touch();
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn to_session_info(&self) -> SessionInfo {
        SessionInfo {
            session_id: self.session_id.clone(),
            created_at: self.created_at,
            last_accessed: self.last_accessed,
            message_count: self.messages.len(),
            expires_at: self.expires_at,
        }
    }
}

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub cleanup_interval_secs: u64,
    pub default_ttl_hours: u32,
}

impl SessionManager {
    pub fn new(default_ttl_hours: u32, cleanup_interval_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval_secs,
            default_ttl_hours,
        }
    }

    pub fn start_cleanup_task(&self) {
        let sessions = self.sessions.clone();
        let interval = self.cleanup_interval_secs;

        tokio::spawn(async move {
            let mut tick = tokio::time::interval(
                tokio::time::Duration::from_secs(interval),
            );
            loop {
                tick.tick().await;
                let mut guard = sessions.write().await;
                let expired: Vec<String> = guard
                    .iter()
                    .filter(|(_, s)| s.is_expired())
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in &expired {
                    guard.remove(id);
                    info!("Cleaned up expired session: {id}");
                }
            }
        });
        info!("Started session cleanup task (interval: {}s)", interval);
    }

    pub async fn get_or_create_session(&self, session_id: &str) -> Session {
        let mut guard = self.sessions.write().await;
        if let Some(session) = guard.get_mut(session_id) {
            if session.is_expired() {
                info!("Session {session_id} expired, creating new session");
                let new_session = Session::new(session_id.to_string());
                guard.insert(session_id.to_string(), new_session.clone());
                new_session
            } else {
                session.touch();
                session.clone()
            }
        } else {
            let session = Session::new(session_id.to_string());
            guard.insert(session_id.to_string(), session.clone());
            info!("Created new session: {session_id}");
            session
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut guard = self.sessions.write().await;
        if let Some(session) = guard.get_mut(session_id) {
            if session.is_expired() {
                guard.remove(session_id);
                info!("Removed expired session: {session_id}");
                None
            } else {
                session.touch();
                Some(session.clone())
            }
        } else {
            None
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> bool {
        let mut guard = self.sessions.write().await;
        if guard.remove(session_id).is_some() {
            info!("Deleted session: {session_id}");
            true
        } else {
            false
        }
    }

    pub async fn list_sessions(&self) -> SessionListResponse {
        let mut guard = self.sessions.write().await;
        // Clean expired first
        let expired: Vec<String> = guard
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            guard.remove(id);
        }
        let sessions: Vec<SessionInfo> = guard.values().map(|s| s.to_session_info()).collect();
        let total = sessions.len();
        SessionListResponse { sessions, total }
    }

    /// Process messages for a request, handling both stateless and session modes.
    /// Returns (all_messages, actual_session_id).
    pub async fn process_messages(
        &self,
        messages: &[Message],
        session_id: Option<&str>,
    ) -> (Vec<Message>, Option<String>) {
        match session_id {
            None => (messages.to_vec(), None),
            Some(sid) => {
                let mut guard = self.sessions.write().await;
                let session = guard
                    .entry(sid.to_string())
                    .or_insert_with(|| Session::new(sid.to_string()));

                if session.is_expired() {
                    *session = Session::new(sid.to_string());
                }

                session.add_messages(messages);
                let all = session.messages.clone();
                info!(
                    "Session {sid}: processing {} new messages, {} total",
                    messages.len(),
                    all.len()
                );
                (all, Some(sid.to_string()))
            }
        }
    }

    pub async fn add_assistant_response(&self, session_id: &str, message: Message) {
        let mut guard = self.sessions.write().await;
        if let Some(session) = guard.get_mut(session_id) {
            session.add_messages(&[message]);
            info!("Added assistant response to session {session_id}");
        }
    }

    pub async fn get_stats(&self) -> HashMap<String, usize> {
        let guard = self.sessions.read().await;
        let active = guard.values().filter(|s| !s.is_expired()).count();
        let expired = guard.values().filter(|s| s.is_expired()).count();
        let total_messages: usize = guard.values().map(|s| s.messages.len()).sum();

        let mut stats = HashMap::new();
        stats.insert("active_sessions".to_string(), active);
        stats.insert("expired_sessions".to_string(), expired);
        stats.insert("total_messages".to_string(), total_messages);
        stats
    }

    pub async fn shutdown(&self) {
        let mut guard = self.sessions.write().await;
        guard.clear();
        info!("Session manager shutdown complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> Message {
        Message { role: role.into(), content: content.into(), name: None, thinking: None }
    }

    #[test]
    fn test_session_new() {
        let s = Session::new("s1".into());
        assert_eq!(s.session_id, "s1");
        assert!(s.messages.is_empty());
        assert!(!s.is_expired());
    }

    #[test]
    fn test_session_add_messages() {
        let mut s = Session::new("s1".into());
        s.add_messages(&[msg("user", "hi")]);
        assert_eq!(s.messages.len(), 1);
        s.add_messages(&[msg("assistant", "hello")]);
        assert_eq!(s.messages.len(), 2);
    }

    #[test]
    fn test_session_to_info() {
        let mut s = Session::new("s1".into());
        s.add_messages(&[msg("user", "hi"), msg("assistant", "hello")]);
        let info = s.to_session_info();
        assert_eq!(info.session_id, "s1");
        assert_eq!(info.message_count, 2);
    }

    #[test]
    fn test_session_expiry() {
        let mut s = Session::new("s1".into());
        s.expires_at = Utc::now() - Duration::seconds(1);
        assert!(s.is_expired());
    }

    #[test]
    fn test_session_touch_extends_ttl() {
        let mut s = Session::new("s1".into());
        s.expires_at = Utc::now() - Duration::seconds(1);
        assert!(s.is_expired());
        s.touch();
        assert!(!s.is_expired());
    }

    #[tokio::test]
    async fn test_get_or_create_new() {
        let mgr = SessionManager::new(1, 300);
        let s = mgr.get_or_create_session("s1").await;
        assert_eq!(s.session_id, "s1");
    }

    #[tokio::test]
    async fn test_get_or_create_existing() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        let s = mgr.get_or_create_session("s1").await;
        assert_eq!(s.session_id, "s1");
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let mgr = SessionManager::new(1, 300);
        assert!(mgr.get_session("missing").await.is_none());
    }

    #[tokio::test]
    async fn test_get_session_found() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        assert!(mgr.get_session("s1").await.is_some());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        assert!(mgr.delete_session("s1").await);
        assert!(!mgr.delete_session("s1").await);
        assert!(mgr.get_session("s1").await.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        mgr.get_or_create_session("s2").await;
        let list = mgr.list_sessions().await;
        assert_eq!(list.total, 2);
    }

    #[tokio::test]
    async fn test_process_messages_stateless() {
        let mgr = SessionManager::new(1, 300);
        let msgs = vec![msg("user", "hi")];
        let (result, sid) = mgr.process_messages(&msgs, None).await;
        assert_eq!(result.len(), 1);
        assert!(sid.is_none());
    }

    #[tokio::test]
    async fn test_process_messages_with_session() {
        let mgr = SessionManager::new(1, 300);
        let (result1, sid1) = mgr.process_messages(&[msg("user", "hi")], Some("s1")).await;
        assert_eq!(result1.len(), 1);
        assert_eq!(sid1, Some("s1".into()));

        let (result2, _) = mgr.process_messages(&[msg("user", "hello")], Some("s1")).await;
        assert_eq!(result2.len(), 2); // accumulated
    }

    #[tokio::test]
    async fn test_add_assistant_response() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        mgr.add_assistant_response("s1", msg("assistant", "hi")).await;
        let s = mgr.get_session("s1").await.unwrap();
        assert_eq!(s.messages.len(), 1);
        assert_eq!(s.messages[0].role, "assistant");
    }

    #[tokio::test]
    async fn test_get_stats() {
        let mgr = SessionManager::new(1, 300);
        mgr.process_messages(&[msg("user", "hi")], Some("s1")).await;
        mgr.process_messages(&[msg("user", "hey")], Some("s2")).await;
        let stats = mgr.get_stats().await;
        assert_eq!(stats["active_sessions"], 2);
        assert_eq!(stats["total_messages"], 2);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let mgr = SessionManager::new(1, 300);
        mgr.get_or_create_session("s1").await;
        mgr.shutdown().await;
        assert!(mgr.get_session("s1").await.is_none());
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(1, SESSION_CLEANUP_INTERVAL_SECS)
    }
}
