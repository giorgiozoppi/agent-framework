// Copyright (c) Microsoft. All rights reserved.

use crate::{StateManager, WorkflowEvent, WorkflowResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

/// Provides services for an executor during workflow execution
#[async_trait]
pub trait WorkflowContext: Send + Sync {
    /// Add an event to the workflow's output queue
    async fn add_event(&self, event: WorkflowEvent) -> WorkflowResult<()>;

    /// Queue a message to be sent to connected executors
    async fn send_message(&self, message: Value, target_id: Option<&str>) -> WorkflowResult<()>;

    /// Add an output value to the workflow's output queue
    async fn yield_output(&self, output: Value) -> WorkflowResult<()>;

    /// Request to halt workflow execution at the end of current super step
    async fn request_halt(&self) -> WorkflowResult<()>;

    /// Read a state value from the workflow's state store (as JSON)
    async fn read_state_json(&self, key: &str, scope: Option<&str>) -> WorkflowResult<Option<Value>>;

    /// Queue a state update for the next super step (as JSON)
    async fn queue_state_update_json(
        &self,
        key: &str,
        value: Value,
        scope: Option<&str>,
    ) -> WorkflowResult<()>;

    /// Read all state keys within the specified scope
    async fn read_state_keys(&self, scope: Option<&str>) -> WorkflowResult<HashSet<String>>;

    /// Queue clearing a scope
    async fn queue_clear_scope(&self, scope: Option<&str>) -> WorkflowResult<()>;

    /// Get trace context for current execution
    fn trace_context(&self) -> Option<&HashMap<String, String>>;

    /// Check if concurrent runs are enabled
    fn concurrent_runs_enabled(&self) -> bool;
}

/// Concrete implementation of WorkflowContext
pub struct WorkflowContextImpl {
    executor_id: String,
    default_scope: String,
    state_manager: StateManager,
    event_sender: mpsc::UnboundedSender<WorkflowEvent>,
    message_sender: mpsc::UnboundedSender<OutgoingMessage>,
    output_sender: mpsc::UnboundedSender<Value>,
    halt_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    trace_context: Option<HashMap<String, String>>,
    concurrent_runs_enabled: bool,
}

impl WorkflowContextImpl {
    pub fn new(
        executor_id: String,
        state_manager: StateManager,
        event_sender: mpsc::UnboundedSender<WorkflowEvent>,
        message_sender: mpsc::UnboundedSender<OutgoingMessage>,
        output_sender: mpsc::UnboundedSender<Value>,
        halt_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let default_scope = executor_id.clone();

        Self {
            executor_id,
            default_scope,
            state_manager,
            event_sender,
            message_sender,
            output_sender,
            halt_requested,
            trace_context: None,
            concurrent_runs_enabled: false,
        }
    }

    pub fn with_trace_context(mut self, trace_context: HashMap<String, String>) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_concurrent_runs(mut self, enabled: bool) -> Self {
        self.concurrent_runs_enabled = enabled;
        self
    }

    fn resolve_scope<'a>(&'a self, scope: Option<&'a str>) -> &'a str {
        scope.unwrap_or(&self.default_scope)
    }
}

#[async_trait]
impl WorkflowContext for WorkflowContextImpl {
    async fn add_event(&self, event: WorkflowEvent) -> WorkflowResult<()> {
        self.event_sender
            .send(event)
            .map_err(|_| crate::WorkflowError::async_error("Failed to send event"))?;
        Ok(())
    }

    async fn send_message(&self, message: Value, target_id: Option<&str>) -> WorkflowResult<()> {
        let outgoing = OutgoingMessage {
            message,
            target_id: target_id.map(|s| s.to_string()),
            source_id: self.executor_id.clone(),
        };

        self.message_sender
            .send(outgoing)
            .map_err(|_| crate::WorkflowError::async_error("Failed to send message"))?;
        Ok(())
    }

    async fn yield_output(&self, output: Value) -> WorkflowResult<()> {
        self.output_sender
            .send(output)
            .map_err(|_| crate::WorkflowError::async_error("Failed to send output"))?;
        Ok(())
    }

    async fn request_halt(&self) -> WorkflowResult<()> {
        self.halt_requested
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn read_state_json(&self, key: &str, scope: Option<&str>) -> WorkflowResult<Option<Value>> {
        let scope = self.resolve_scope(scope);
        self.state_manager.read_state::<Value>(scope, key).await
    }

    async fn queue_state_update_json(
        &self,
        key: &str,
        value: Value,
        scope: Option<&str>,
    ) -> WorkflowResult<()> {
        let scope = self.resolve_scope(scope);
        self.state_manager.queue_state_update(scope, key, &value).await
    }

    async fn read_state_keys(&self, scope: Option<&str>) -> WorkflowResult<HashSet<String>> {
        let scope = self.resolve_scope(scope);
        self.state_manager.read_state_keys(scope).await
    }

    async fn queue_clear_scope(&self, scope: Option<&str>) -> WorkflowResult<()> {
        let scope = self.resolve_scope(scope);
        self.state_manager.queue_clear_scope(scope).await
    }

    fn trace_context(&self) -> Option<&HashMap<String, String>> {
        self.trace_context.as_ref()
    }

    fn concurrent_runs_enabled(&self) -> bool {
        self.concurrent_runs_enabled
    }
}

/// Message being sent between executors
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub message: Value,
    pub target_id: Option<String>,
    pub source_id: String,
}