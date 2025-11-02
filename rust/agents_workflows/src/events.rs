// Copyright (c) Microsoft. All rights reserved.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Events emitted during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Unique identifier for this event
    pub id: Uuid,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// The kind of event
    pub kind: WorkflowEventKind,
    /// Optional metadata
    pub metadata: Option<Value>,
}

impl WorkflowEvent {
    pub fn new(kind: WorkflowEventKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            kind,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Types of workflow events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkflowEventKind {
    /// Workflow execution started
    WorkflowStarted {
        workflow_id: String,
        run_id: String,
    },

    /// Workflow execution completed successfully
    WorkflowCompleted {
        workflow_id: String,
        run_id: String,
    },

    /// Workflow execution failed
    WorkflowFailed {
        workflow_id: String,
        run_id: String,
        error: String,
    },

    /// Executor started processing a message
    ExecutorInvoked {
        executor_id: String,
        message_type: String,
    },

    /// Executor completed processing
    ExecutorCompleted {
        executor_id: String,
        success: bool,
    },

    /// Executor failed
    ExecutorFailed {
        executor_id: String,
        error: String,
    },

    /// Workflow output produced
    WorkflowOutput {
        executor_id: String,
        output: Value,
    },

    /// State update occurred
    StateUpdate {
        scope: String,
        key: String,
        operation: StateOperation,
    },

    /// External request needed
    RequestInfo {
        request_id: String,
        request_type: String,
        data: Value,
    },

    /// Request to halt execution
    RequestHalt {
        reason: Option<String>,
    },

    /// Super step started
    SuperStepStarted {
        step_id: String,
    },

    /// Super step completed
    SuperStepCompleted {
        step_id: String,
        message_count: usize,
    },

    /// Warning occurred
    WorkflowWarning {
        message: String,
    },

    /// Subworkflow events
    SubworkflowWarning {
        subworkflow_id: String,
        message: String,
    },

    SubworkflowError {
        subworkflow_id: String,
        error: String,
    },
}

/// State operations for event tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateOperation {
    Read,
    Write,
    Delete,
    Clear,
}