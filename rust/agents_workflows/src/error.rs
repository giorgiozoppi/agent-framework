// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

/// Result type for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Errors that can occur during workflow operations
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Executor not found: {0}")]
    ExecutorNotFound(String),

    #[error("Executor already exists: {0}")]
    ExecutorAlreadyExists(String),

    #[error("Executor is unbound: {0}")]
    ExecutorUnbound(String),

    #[error("Edge already exists from {0} to {1}")]
    EdgeAlreadyExists(String, String),

    #[error("Invalid workflow configuration: {message}")]
    InvalidConfiguration { message: String },

    #[error("Workflow run failed: {message}")]
    RunFailed { message: String },

    #[error("State error: {message}")]
    StateError { message: String },

    #[error("Serialization error: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Agent error: {source}")]
    AgentError {
        #[from]
        source: agents_traits::AgentError,
    },

    #[error("Async error: {message}")]
    AsyncError { message: String },

    #[error("Workflow ownership error: {message}")]
    OwnershipError { message: String },
}

impl WorkflowError {
    pub fn invalid_configuration<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }

    pub fn run_failed<S: Into<String>>(message: S) -> Self {
        Self::RunFailed {
            message: message.into(),
        }
    }

    pub fn state_error<S: Into<String>>(message: S) -> Self {
        Self::StateError {
            message: message.into(),
        }
    }

    pub fn async_error<S: Into<String>>(message: S) -> Self {
        Self::AsyncError {
            message: message.into(),
        }
    }

    pub fn ownership_error<S: Into<String>>(message: S) -> Self {
        Self::OwnershipError {
            message: message.into(),
        }
    }
}