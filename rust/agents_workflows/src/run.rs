// Copyright (c) Microsoft. All rights reserved.

use crate::{WorkflowEvent, WorkflowResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Represents a workflow run that tracks execution status and events
pub struct Run {
    pub run_id: String,
    status: Arc<RwLock<RunStatus>>,
    event_receiver: mpsc::UnboundedReceiver<WorkflowEvent>,
    event_sink: Arc<RwLock<Vec<WorkflowEvent>>>,
    control_handle: RunControlHandle,
}

impl Run {
    pub(crate) fn new(
        run_id: Option<String>,
        event_receiver: mpsc::UnboundedReceiver<WorkflowEvent>,
        control_handle: RunControlHandle,
    ) -> Self {
        let run_id = run_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        Self {
            run_id,
            status: Arc::new(RwLock::new(RunStatus::Running)),
            event_receiver,
            event_sink: Arc::new(RwLock::new(Vec::new())),
            control_handle,
        }
    }

    /// Get the unique identifier for this run
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Get the current execution status
    pub async fn get_status(&self) -> RunStatus {
        *self.status.read().await
    }

    /// Set the run status
    pub(crate) async fn set_status(&self, status: RunStatus) {
        *self.status.write().await = status;
    }

    /// Get all events emitted by the workflow so far
    pub async fn get_events(&self) -> Vec<WorkflowEvent> {
        self.event_sink.read().await.clone()
    }

    /// Wait for the next batch of events
    pub async fn take_events(&mut self) -> WorkflowResult<Vec<WorkflowEvent>> {
        let mut events = Vec::new();

        // Collect all currently available events
        while let Ok(event) = self.event_receiver.try_recv() {
            events.push(event);
        }

        // If no events available, wait for at least one
        if events.is_empty() {
            if let Some(event) = self.event_receiver.recv().await {
                events.push(event);
            }
        }

        // Store events in sink
        {
            let mut sink = self.event_sink.write().await;
            sink.extend(events.clone());
        }

        Ok(events)
    }

    /// Run to the next halt point and return if any events were produced
    pub async fn run_to_next_halt(&mut self) -> WorkflowResult<bool> {
        let events = self.take_events().await?;
        Ok(!events.is_empty())
    }

    /// Resume execution with an external response
    pub async fn resume_with_response(&self, response: serde_json::Value) -> WorkflowResult<()> {
        self.control_handle.send_response(response).await
    }

    /// Request cancellation of the workflow run
    pub async fn cancel(&self) -> WorkflowResult<()> {
        self.control_handle.cancel().await
    }

    /// Wait for the workflow to complete and return final status
    pub async fn wait_for_completion(&self) -> WorkflowResult<RunStatus> {
        self.control_handle.wait_for_completion().await
    }
}

/// Control handle for managing a workflow run
#[derive(Debug, Clone)]
pub struct RunControlHandle {
    response_sender: Arc<mpsc::UnboundedSender<serde_json::Value>>,
    cancel_sender: Arc<mpsc::UnboundedSender<()>>,
    completion_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<RunStatus>>>>,
}

impl RunControlHandle {
    pub fn new(
        response_sender: mpsc::UnboundedSender<serde_json::Value>,
        cancel_sender: mpsc::UnboundedSender<()>,
        completion_receiver: mpsc::UnboundedReceiver<RunStatus>,
    ) -> Self {
        Self {
            response_sender: Arc::new(response_sender),
            cancel_sender: Arc::new(cancel_sender),
            completion_receiver: Arc::new(RwLock::new(Some(completion_receiver))),
        }
    }

    async fn send_response(&self, response: serde_json::Value) -> WorkflowResult<()> {
        self.response_sender
            .send(response)
            .map_err(|_| crate::WorkflowError::async_error("Failed to send response"))?;
        Ok(())
    }

    async fn cancel(&self) -> WorkflowResult<()> {
        self.cancel_sender
            .send(())
            .map_err(|_| crate::WorkflowError::async_error("Failed to send cancel signal"))?;
        Ok(())
    }

    async fn wait_for_completion(&self) -> WorkflowResult<RunStatus> {
        let mut receiver_guard = self.completion_receiver.write().await;
        if let Some(mut receiver) = receiver_guard.take() {
            receiver
                .recv()
                .await
                .ok_or_else(|| crate::WorkflowError::async_error("Completion channel closed"))
        } else {
            Err(crate::WorkflowError::async_error(
                "Completion already awaited",
            ))
        }
    }
}

/// Status of a workflow run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    /// Run is currently executing
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed with an error
    Failed,
    /// Run was cancelled
    Cancelled,
    /// Run is paused waiting for external input
    WaitingForInput,
    /// Run has been halted
    Halted,
}

impl RunStatus {
    /// Check if the run is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled)
    }

    /// Check if the run is still active
    pub fn is_active(&self) -> bool {
        matches!(self, RunStatus::Running | RunStatus::WaitingForInput)
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Running => write!(f, "Running"),
            RunStatus::Completed => write!(f, "Completed"),
            RunStatus::Failed => write!(f, "Failed"),
            RunStatus::Cancelled => write!(f, "Cancelled"),
            RunStatus::WaitingForInput => write!(f, "Waiting for Input"),
            RunStatus::Halted => write!(f, "Halted"),
        }
    }
}

/// Information about a super step in execution
#[derive(Debug, Clone)]
pub struct SuperStepInfo {
    pub step_id: String,
    pub message_count: usize,
    pub executor_ids: Vec<String>,
    pub duration: Option<std::time::Duration>,
}

impl SuperStepInfo {
    pub fn new(step_id: String) -> Self {
        Self {
            step_id,
            message_count: 0,
            executor_ids: Vec::new(),
            duration: None,
        }
    }

    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    pub fn with_executors(mut self, executor_ids: Vec<String>) -> Self {
        self.executor_ids = executor_ids;
        self
    }

    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}