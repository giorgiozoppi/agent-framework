// Copyright (c) Microsoft. All rights reserved.

use crate::{AgentRunResponse, AgentThread, ChatMessage, Result};
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;
use uuid::Uuid;

/// Metadata about an agent
#[derive(Debug, Clone)]
pub struct AIAgentMetadata {
    /// Unique identifier for the agent
    pub id: String,
    /// Human-readable name
    pub name: Option<String>,
    /// Description of the agent's purpose
    pub description: Option<String>,
}

impl AIAgentMetadata {
    /// Create new agent metadata
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
        }
    }

    /// Set the agent name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the agent description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get display name (name or id)
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

impl Default for AIAgentMetadata {
    fn default() -> Self {
        Self::new(Uuid::new_v4().to_string())
    }
}

/// Options for agent execution
#[derive(Debug, Clone, Default)]
pub struct AgentRunOptions {
    /// Whether to allow background responses
    pub allow_background_responses: bool,
    /// Continuation token for polling background responses
    pub continuation_token: Option<String>,
    /// Maximum number of function call iterations
    pub max_iterations: Option<u32>,
    /// Additional provider-specific options
    pub additional_properties: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Type alias for streaming response updates
pub type AgentResponseStream = Pin<Box<dyn Stream<Item = Result<crate::AgentRunResponseUpdate>> + Send>>;

/// Base trait for all AI agents
///
/// Provides the core interface for agent interactions and conversation management.
/// An agent instance may participate in multiple concurrent conversations, and each
/// conversation may involve multiple agents working together.
#[async_trait]
pub trait AIAgent: Send + Sync {
    /// Get the agent metadata (id, name, description)
    fn metadata(&self) -> &AIAgentMetadata;

    /// Get the unique identifier for this agent
    fn id(&self) -> &str {
        &self.metadata().id
    }

    /// Get the agent name
    fn name(&self) -> Option<&str> {
        self.metadata().name.as_deref()
    }

    /// Get the agent description
    fn description(&self) -> Option<&str> {
        self.metadata().description.as_deref()
    }

    /// Get a display-friendly name (name or id)
    fn display_name(&self) -> &str {
        self.metadata().display_name()
    }

    /// Create a new conversation thread compatible with this agent
    fn get_new_thread(&self) -> Box<dyn AgentThread>;

    /// Deserialize a thread from JSON
    fn deserialize_thread(&self, serialized: Value) -> Result<Box<dyn AgentThread>>;

    /// Run the agent with no new messages (uses existing thread context)
    async fn run(
        &self,
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentRunResponse> {
        self.run_with_messages(&[], thread, options).await
    }

    /// Run the agent with a text message
    async fn run_with_text<S: Into<String> + Send>(
        &self,
        message: S,
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentRunResponse> {
        let msg = ChatMessage::user(message);
        self.run_with_message(msg, thread, options).await
    }

    /// Run the agent with a single message
    async fn run_with_message(
        &self,
        message: ChatMessage,
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentRunResponse> {
        self.run_with_messages(&[message], thread, options).await
    }

    /// Run the agent with multiple messages (core method)
    async fn run_with_messages(
        &self,
        messages: &[ChatMessage],
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentRunResponse>;

    /// Run the agent with streaming responses
    async fn run_streaming(
        &self,
        messages: &[ChatMessage],
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentResponseStream>;

    /// Get a service from the agent
    fn get_service(&self, service_type: &str) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        let _ = service_type;
        None
    }
}
