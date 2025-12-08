//! # Actor-Based Coordination System
//!
//! This module provides a robust actor-based coordination system for RustyDB,
//! enabling asynchronous message passing, supervision, and fault tolerance.
//!
//! ## Features
//!
//! - **Lightweight Actors**: Efficient async actors with mailboxes
//! - **Supervision Trees**: Hierarchical supervision with restart strategies
//! - **Message Routing**: Pattern-based message routing and broadcasts
//! - **Backpressure**: Automatic flow control with bounded mailboxes
//! - **Actor Discovery**: Name-based actor registration and lookup
//! - **Lifecycle Management**: Automatic cleanup and resource management
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │           Actor System Supervisor               │
//! └─────────────────┬───────────────────────────────┘
//!                   │
//!       ┌───────────┼───────────┬──────────────┐
//!       │           │           │              │
//!   ┌───▼───┐   ┌──▼────┐  ┌───▼────┐    ┌────▼────┐
//!   │Actor A│   │Actor B│  │Actor C │    │Actor D  │
//!   │(Worker)   │(Worker)  │(Worker)│    │(Supervisor)
//!   └───────┘   └───────┘  └───────┘    └─────┬─────┘
//!                                              │
//!                                         ┌────┴────┐
//!                                         │         │
//!                                     ┌───▼───┐ ┌──▼────┐
//!                                     │Actor E│ │Actor F│
//!                                     └───────┘ └───────┘
//! ```

use std::any::Any;
use std::collections::HashMap;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::error::Result;

/// Unique identifier for an actor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorId(u64);

impl ActorId {
    /// Generate a new unique actor ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Actor({})", self.0)
    }
}

/// Actor address for sending messages
#[derive(Clone)]
pub struct ActorRef {
    id: ActorId,
    name: Option<String>,
    tx: mpsc::Sender<ActorMessage>,
}

impl ActorRef {
    /// Create a new actor reference
    pub fn new(id: ActorId, name: Option<String>, tx: mpsc::Sender<ActorMessage>) -> Self {
        Self { id, name, tx }
    }

    /// Get the actor ID
    pub fn id(&self) -> ActorId {
        self.id
    }

    /// Get the actor name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Send a message to this actor
    pub async fn send<M: Message>(&self, msg: M) -> Result<()> {
        let envelope = ActorMessage::User(Box::new(msg));
        self.tx
            .send(envelope)
            .await
            .map_err(|_| DbError::Internal("Actor mailbox closed".into()))?;
        Ok(())
    }

    /// Send a message and wait for a response
    pub async fn ask<M: Message, R: 'static + Send>(
        &self,
        msg: M,
        timeout_duration: Duration,
    ) -> Result<R> {
        let (tx, rx) = oneshot::channel();
        let envelope = ActorMessage::Request {
            msg: Box::new(msg),
            reply: tx,
        };

        self.tx
            .send(envelope)
            .await
            .map_err(|_| DbError::Internal("Actor mailbox closed".into()))?;

        let response = timeout(timeout_duration, rx)
            .await
            .map_err(|_| DbError::Internal("Request timeout".into()))?
            .map_err(|_| DbError::Internal("Response channel closed".into()))?;

        response
            .downcast::<R>()
            .map(|boxed| *boxed)
            .map_err(|_| DbError::Internal("Invalid response type".into()))
    }

    /// Stop the actor
    pub async fn stop(&self) -> Result<()> {
        self.tx
            .send(ActorMessage::Stop)
            .await
            .map_err(|_| DbError::Internal("Actor mailbox closed".into()))?;
        Ok(())
    }
}

impl fmt::Debug for ActorRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorRef")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

/// Trait for messages that can be sent to actors
pub trait Message: Send + 'static {
    /// Type-erased message
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + 'static> Message for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Internal actor message envelope
pub enum ActorMessage {
    /// User-defined message
    User(Box<dyn Any + Send>),
    /// Request-response message
    Request {
        msg: Box<dyn Any + Send>,
        reply: oneshot::Sender<Box<dyn Any + Send>>,
    },
    /// System message to stop the actor
    Stop,
    /// System message to restart the actor
    Restart,
}

/// Actor trait that defines actor behavior
#[async_trait::async_trait]
pub trait Actor: Send + 'static {
    /// Called when the actor is started
    async fn started(&mut self, _ctx: &ActorContext) -> Result<()> {
        Ok(())
    }

    /// Called when the actor is stopped
    async fn stopped(&mut self, _ctx: &ActorContext) -> Result<()> {
        Ok(())
    }

    /// Handle a message
    async fn handle(&mut self, msg: Box<dyn Any + Send>, ctx: &ActorContext) -> Result<()>;

    /// Handle a request-response message
    async fn handle_request(
        &mut self,
        msg: Box<dyn Any + Send>,
        _ctx: &ActorContext,
    ) -> Result<Box<dyn Any + Send>> {
        Err(DbError::Internal(format!(
            "Request not implemented for message: {:?}",
            msg.type_id()
        )))
    }
}

/// Actor execution context
pub struct ActorContext {
    id: ActorId,
    name: Option<String>,
    system: Arc<ActorSystem>,
    self_ref: Option<ActorRef>,
}

impl ActorContext {
    /// Create a new actor context
    pub fn new(id: ActorId, name: Option<String>, system: Arc<ActorSystem>) -> Self {
        Self {
            id,
            name,
            system,
            self_ref: None,
        }
    }

    /// Set the self reference
    pub fn set_self_ref(&mut self, actor_ref: ActorRef) {
        self.self_ref = Some(actor_ref);
    }

    /// Get the actor ID
    pub fn id(&self) -> ActorId {
        self.id
    }

    /// Get the actor name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get a reference to self
    pub fn self_ref(&self) -> Option<&ActorRef> {
        self.self_ref.as_ref()
    }

    /// Get the actor system
    pub fn system(&self) -> &Arc<ActorSystem> {
        &self.system
    }

    /// Spawn a new child actor
    pub async fn spawn<A: Actor>(
        &self,
        actor: A,
        name: Option<String>,
        mailbox_size: usize,
    ) -> Result<ActorRef> {
        self.system.spawn(actor, name, mailbox_size).await
    }

    /// Stop self
    pub async fn stop(&self) -> Result<()> {
        if let Some(self_ref) = &self.self_ref {
            self_ref.stop().await
        } else {
            Err(DbError::Internal("No self reference available".into()))
        }
    }
}

impl fmt::Debug for ActorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorContext")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

/// Supervision strategy for actor failures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisionStrategy {
    /// Restart the failed actor
    Restart,
    /// Escalate the failure to the supervisor
    Escalate,
    /// Stop the failed actor
    Stop,
    /// Resume the actor (ignore the error)
    Resume,
}

/// Actor supervisor configuration
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Supervision strategy
    pub strategy: SupervisionStrategy,
    /// Maximum number of restarts within the time window
    pub max_restarts: usize,
    /// Time window for restart counting (seconds)
    pub time_window_secs: u64,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            strategy: SupervisionStrategy::Restart,
            max_restarts: 3,
            time_window_secs: 60,
        }
    }
}

/// Actor system that manages all actors
pub struct ActorSystem {
    /// Registry of all active actors
    actors: RwLock<HashMap<ActorId, ActorHandle>>,
    /// Named actor registry
    named_actors: RwLock<HashMap<String, ActorId>>,
    /// Supervisor configuration
    supervisor_config: Mutex<SupervisorConfig>,
    /// System shutdown signal
    shutdown_tx: Arc<Mutex<Option<tokio::sync::broadcast::Sender<()>>>>,
}

/// Handle to an actor for lifecycle management
struct ActorHandle {
    id: ActorId,
    name: Option<String>,
    actor_ref: ActorRef,
    join_handle: tokio::task::JoinHandle<()>,
}

impl ActorSystem {
    /// Create a new actor system
    pub fn new() -> Arc<Self> {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        Arc::new(Self {
            actors: RwLock::new(HashMap::new()),
            named_actors: RwLock::new(HashMap::new()),
            supervisor_config: Mutex::new(SupervisorConfig::default()),
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        })
    }

    /// Configure supervision strategy
    pub async fn configure_supervision(&self, config: SupervisorConfig) {
        *self.supervisor_config.lock().await = config;
    }

    /// Spawn a new actor
    pub async fn spawn<A: Actor>(
        self: &Arc<Self>,
        mut actor: A,
        name: Option<String>,
        mailbox_size: usize,
    ) -> Result<ActorRef> {
        let id = ActorId::new();
        let (tx, mut rx) = mpsc::channel::<ActorMessage>(mailbox_size);

        let mut ctx = ActorContext::new(id, name.clone(), Arc::clone(self));
        let actor_ref = ActorRef::new(id, name.clone(), tx);
        ctx.set_self_ref(actor_ref.clone());

        // Start the actor
        if let Err(e) = actor.started(&ctx).await {
            error!("Failed to start actor {}: {}", id, e);
            return Err(e);
        }

        info!("Starting actor: {}", id);

        // Spawn the actor task
        let system = Arc::clone(self);
        let actor_ref_clone = actor_ref.clone();
        let join_handle = tokio::spawn(async move {
            let mut shutdown_rx = {
                let guard = system.shutdown_tx.lock().await;
                guard.as_ref().map(|tx| tx.subscribe())
            };

            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        match msg {
                            ActorMessage::User(msg) => {
                                if let Err(e) = actor.handle(msg, &ctx).await {
                                    error!("Actor {} error handling message: {}", id, e);
                                    // Apply supervision strategy
                                    system.handle_actor_failure(id, e).await;
                                }
                            }
                            ActorMessage::Request { msg, reply } => {
                                match actor.handle_request(msg, &ctx).await {
                                    Ok(response) => {
                                        let _ = reply.send(response);
                                    }
                                    Err(e) => {
                                        error!("Actor {} error handling request: {}", id, e);
                                        system.handle_actor_failure(id, e).await;
                                    }
                                }
                            }
                            ActorMessage::Stop => {
                                info!("Stopping actor: {}", id);
                                if let Err(e) = actor.stopped(&ctx).await {
                                    error!("Error stopping actor {}: {}", id, e);
                                }
                                break;
                            }
                            ActorMessage::Restart => {
                                info!("Restarting actor: {}", id);
                                if let Err(e) = actor.stopped(&ctx).await {
                                    error!("Error stopping actor {} for restart: {}", id, e);
                                }
                                if let Err(e) = actor.started(&ctx).await {
                                    error!("Error restarting actor {}: {}", id, e);
                                    break;
                                }
                            }
                        }
                    }
                    _ = async {
                        if let Some(ref mut rx) = shutdown_rx {
                            rx.recv().await.ok();
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {
                        info!("Actor {} received shutdown signal", id);
                        if let Err(e) = actor.stopped(&ctx).await {
                            error!("Error stopping actor {} on shutdown: {}", id, e);
                        }
                        break;
                    }
                    else => {
                        debug!("Actor {} mailbox closed", id);
                        break;
                    }
                }
            }

            // Unregister actor
            system.unregister_actor(id).await;
        });

        // Register the actor
        let handle = ActorHandle {
            id,
            name: name.clone(),
            actor_ref: actor_ref.clone(),
            join_handle,
        };

        let mut actors = self.actors.write().await;
        actors.insert(id, handle);

        if let Some(name) = name {
            let mut named = self.named_actors.write().await;
            named.insert(name, id);
        }

        Ok(actor_ref_clone)
    }

    /// Find an actor by name
    pub async fn find_actor(&self, name: &str) -> Option<ActorRef> {
        let named = self.named_actors.read().await;
        let id = named.get(name)?;
        let actors = self.actors.read().await;
        actors.get(id).map(|h| h.actor_ref.clone())
    }

    /// Get an actor by ID
    pub async fn get_actor(&self, id: ActorId) -> Option<ActorRef> {
        let actors = self.actors.read().await;
        actors.get(&id).map(|h| h.actor_ref.clone())
    }

    /// List all active actors
    pub async fn list_actors(&self) -> Vec<(ActorId, Option<String>)> {
        let actors = self.actors.read().await;
        actors
            .values()
            .map(|h| (h.id, h.name.clone()))
            .collect()
    }

    /// Broadcast a message to all actors
    pub async fn broadcast<M: Message + Clone>(&self, msg: M) -> Result<()> {
        let actors = self.actors.read().await;
        let mut errors = Vec::new();

        for handle in actors.values() {
            if let Err(e) = handle.actor_ref.send(msg.clone()).await {
                errors.push((handle.id, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            log::warn!("Broadcast failed for {} actors", errors.len());
            Err(DbError::Internal(format!(
                "Broadcast failed for {} actors",
                errors.len()
            )))
        }
    }

    /// Handle actor failure according to supervision strategy
    async fn handle_actor_failure(&self, id: ActorId, error: DbError) {
        let config = self.supervisor_config.lock().await;
        let strategy = config.strategy;
        drop(config);

        match strategy {
            SupervisionStrategy::Restart => {
                info!("Restarting failed actor: {}", id);
                if let Some(actor_ref) = self.get_actor(id).await {
                    if let Err(e) = actor_ref
                        .tx
                        .send(ActorMessage::Restart)
                        .await
                    {
                        error!("Failed to send restart message to actor {}: {}", id, e);
                    }
                }
            }
            SupervisionStrategy::Stop => {
                info!("Stopping failed actor: {}", id);
                if let Some(actor_ref) = self.get_actor(id).await {
                    let _ = actor_ref.stop().await;
                }
            }
            SupervisionStrategy::Escalate => {
                error!("Escalating failure from actor {}: {}", id, error);
                // In a real system, this would notify a parent supervisor
            }
            SupervisionStrategy::Resume => {
                log::warn!("Resuming actor {} after error: {}", id, error);
                // Continue processing
            }
        }
    }

    /// Unregister an actor
    async fn unregister_actor(&self, id: ActorId) {
        let mut actors = self.actors.write().await;
        if let Some(handle) = actors.remove(&id) {
            if let Some(name) = handle.name {
                let mut named = self.named_actors.write().await;
                named.remove(&name);
            }
            debug!("Unregistered actor: {}", id);
        }
    }

    /// Shutdown the entire actor system
    pub async fn shutdown(self: &Arc<Self>) -> Result<()> {
        info!("Shutting down actor system...");

        // Send shutdown signal
        {
            let mut guard = self.shutdown_tx.lock().await;
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }

        // Wait for all actors to stop
        let handles: Vec<_> = {
            let mut actors = self.actors.write().await;
            actors
                .drain()
                .map(|(_, h)| h.join_handle)
                .collect()
        };

        for handle in handles {
            if let Err(e) = handle.await {
                error!("Error waiting for actor to stop: {}", e);
            }
        }

        // Clear named actors
        {
            let mut named = self.named_actors.write().await;
            named.clear();
        }

        info!("Actor system shutdown complete");
        Ok(())
    }

    /// Get statistics about the actor system
    pub async fn statistics(&self) -> ActorSystemStats {
        let actors = self.actors.read().await;
        ActorSystemStats {
            total_actors: actors.len(),
            named_actors: actors.values().filter(|h| h.name.is_some()).count(),
        }
    }
}

impl Default for ActorSystem {
    fn default() -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        Self {
            actors: RwLock::new(HashMap::new()),
            named_actors: RwLock::new(HashMap::new()),
            supervisor_config: Mutex::new(SupervisorConfig::default()),
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        }
    }
}

/// Statistics about the actor system
#[derive(Debug, Clone)]
pub struct ActorSystemStats {
    /// Total number of active actors
    pub total_actors: usize,
    /// Number of named actors
    pub named_actors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestActor {
        counter: usize,
    }

    #[async_trait::async_trait]
    impl Actor for TestActor {
        async fn started(&mut self, ctx: &ActorContext) -> Result<()> {
            info!("TestActor started: {:?}", ctx.id());
            Ok(())
        }

        async fn stopped(&mut self, ctx: &ActorContext) -> Result<()> {
            info!("TestActor stopped: {:?}", ctx.id());
            Ok(())
        }

        async fn handle(&mut self, msg: Box<dyn Any + Send>, _ctx: &ActorContext) -> Result<()> {
            if let Some(count) = msg.downcast_ref::<usize>() {
                self.counter += count;
                Ok(())
            } else {
                Err(DbError::Internal("Unknown message type".into()))
            }
        }

        async fn handle_request(
            &mut self,
            msg: Box<dyn Any + Send>,
            _ctx: &ActorContext,
        ) -> Result<Box<dyn Any + Send>> {
            if msg.downcast_ref::<String>().is_some() {
                Ok(Box::new(self.counter))
            } else {
                Err(DbError::Internal("Unknown request type".into()))
            }
        }
    }

    #[tokio::test]
    async fn test_actor_spawn_and_send() {
        let system = ActorSystem::new();
        let actor = TestActor { counter: 0 };

        let actor_ref = system
            .spawn(actor, Some("test-actor".into()), 10)
            .await
            .unwrap();

        actor_ref.send(5usize).await.unwrap();
        actor_ref.send(3usize).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let result: usize = actor_ref
            .ask("get".to_string(), Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(result, 8);

        system.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_actor_find_by_name() {
        let system = ActorSystem::new();
        let actor = TestActor { counter: 0 };

        system
            .spawn(actor, Some("named-actor".into()), 10)
            .await
            .unwrap();

        let found = system.find_actor("named-actor").await;
        assert!(found.is_some());

        system.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_actor_broadcast() {
        let system = ActorSystem::new();

        for i in 0..5 {
            let actor = TestActor { counter: 0 };
            system
                .spawn(actor, Some(format!("actor-{}", i)), 10)
                .await
                .unwrap();
        }

        system.broadcast(10usize).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        for i in 0..5 {
            let actor_ref = system.find_actor(&format!("actor-{}", i)).await.unwrap();
            let result: usize = actor_ref
                .ask("get".to_string(), Duration::from_secs(1))
                .await
                .unwrap();
            assert_eq!(result, 10);
        }

        system.shutdown().await.unwrap();
    }
}


