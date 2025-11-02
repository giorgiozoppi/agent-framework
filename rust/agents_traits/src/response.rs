// Copyright (c) Microsoft. All rights reserved.

use crate::{ChatMessage, UsageDetails};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the response to an agent run request
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRunResponse {
    /// The response messages
    pub messages: Vec<ChatMessage>,

    /// Identifier of the agent that generated this response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

    /// Unique identifier for this response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,

    /// Timestamp when the response was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageDetails>,

    /// Continuation token for background responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,

    /// Additional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<HashMap<String, serde_json::Value>>,
}

impl Clone for AgentRunResponse {
    fn clone(&self) -> Self {
        Self {
            messages: self.messages.clone(),
            agent_id: self.agent_id.clone(),
            response_id: self.response_id.clone(),
            created_at: self.created_at,
            usage: self.usage.clone(),
            continuation_token: self.continuation_token.clone(),
            additional_properties: self.additional_properties.clone(),
        }
    }
}

impl AgentRunResponse {
    /// Create a new response with a single message
    pub fn new(message: ChatMessage) -> Self {
        Self {
            messages: vec![message],
            agent_id: None,
            response_id: None,
            created_at: Some(Utc::now()),
            usage: None,
            continuation_token: None,
            additional_properties: None,
        }
    }

    /// Create a new response with multiple messages
    pub fn with_messages(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            agent_id: None,
            response_id: None,
            created_at: Some(Utc::now()),
            usage: None,
            continuation_token: None,
            additional_properties: None,
        }
    }

    /// Get the concatenated text from all messages
    pub fn text(&self) -> String {
        self.messages
            .iter()
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert to streaming updates
    pub fn to_updates(&self) -> Vec<AgentRunResponseUpdate> {
        let mut updates = Vec::new();

        for message in &self.messages {
            updates.push(AgentRunResponseUpdate {
                message: Some(message.clone()),
                agent_id: self.agent_id.clone(),
                response_id: self.response_id.clone(),
                created_at: self.created_at,
                usage: None,
                additional_properties: None,
            });
        }

        // Add usage as final update if present
        if self.usage.is_some() {
            updates.push(AgentRunResponseUpdate {
                message: None,
                agent_id: self.agent_id.clone(),
                response_id: self.response_id.clone(),
                created_at: self.created_at,
                usage: self.usage.clone(),
                additional_properties: self.additional_properties.clone(),
            });
        }

        updates
    }
}

impl std::fmt::Display for AgentRunResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text())
    }
}

/// Represents an incremental update during streaming agent responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResponseUpdate {
    /// The message update (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ChatMessage>,

    /// Identifier of the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

    /// Response identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,

    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    /// Usage information (typically in final update)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageDetails>,

    /// Additional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<HashMap<String, serde_json::Value>>,
}

impl AgentRunResponseUpdate {
    /// Create a new empty response update
    pub fn new() -> Self {
        Self {
            message: None,
            agent_id: None,
            response_id: None,
            created_at: Some(Utc::now()),
            usage: None,
            additional_properties: None,
        }
    }
}

impl Default for AgentRunResponseUpdate {
    fn default() -> Self {
        Self::new()
    }
}
