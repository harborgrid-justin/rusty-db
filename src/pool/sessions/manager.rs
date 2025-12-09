// Main session manager
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::Result;
use super::state::{SID, SessionState};
use super::auth::AuthenticationProvider;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_sessions: usize,
    pub idle_timeout_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            idle_timeout_secs: 3600,
        }
    }
}

pub struct SessionManager {
    config: SessionConfig,
    sessions: Arc<RwLock<HashMap<SID, SessionState>>>,
    auth_provider: Arc<AuthenticationProvider>,
    next_id: std::sync::atomic::AtomicU64,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_provider: Arc::new(AuthenticationProvider::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    pub async fn create_session(
        &mut self,
        username: &str,
        _password: &str,
        _schema: Option<String>,
    ) -> Result<SID> {
        let sid = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let schema = username.to_string();
        let state = SessionState::new(sid, username.to_string(), schema);
        self.sessions.write().insert(sid, state);
        Ok(sid)
    }

    pub async fn get_session(&self, sid: SID) -> Option<SessionState> {
        self.sessions.read().get(&sid).cloned()
    }

    pub async fn terminate_session(&mut self, sid: SID, _immediate: bool) -> Result<()> {
        self.sessions.write().remove(&sid);
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<SID> {
        self.sessions.read().keys().copied().collect()
    }

    pub async fn set_session_variable(
        &mut self,
        sid: SID,
        name: &str,
        value: crate::common::Value,
    ) -> Result<()> {
        if let Some(session) = self.sessions.write().get_mut(&sid) {
            session.session_variables.insert(name.to_string(), value);
        }
        Ok(())
    }

    pub async fn set_cpu_limit(&mut self, _sid: SID, _limit_ms: u64) -> Result<()> {
        Ok(())
    }
}
