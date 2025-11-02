// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Errors that can occur during agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    /// Error during agent execution
    #[error("Agent execution error: {0}")]
    ExecutionError(String),

    /// Error during serialization/deserialization
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Error during thread operations
    #[error("Thread error: {0}")]
    ThreadError(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Service not found
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    /// Service error
    #[error("Service error: {0}")]
    ServiceError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for AgentError {
    fn from(s: String) -> Self {
        AgentError::Other(s)
    }
}

impl From<&str> for AgentError {
    fn from(s: &str) -> Self {
        AgentError::Other(s.to_string())
    }
}
