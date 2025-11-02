// Copyright (c) Microsoft. All rights reserved.

use crate::{ChatMessage, Result};
use async_trait::async_trait;
use serde_json::Value;

/// Base trait for all agent threads
///
/// An `AgentThread` contains the state of a specific conversation with an agent which may include:
/// - Conversation history or a reference to externally stored conversation history
/// - Memories or a reference to externally stored memories
/// - Any other state that the agent needs to persist across runs for a conversation
///
/// An `AgentThread` is always constructed by an `AIAgent` so that the `AIAgent`
/// can attach any necessary behaviors to the `AgentThread`.
#[async_trait]
pub trait AgentThread: Send + Sync {
    /// Serialize the thread state to JSON
    fn serialize(&self) -> Result<Value>;

    /// Called when new messages have been contributed to the chat
    async fn messages_received(&mut self, new_messages: &[ChatMessage]) -> Result<()> {
        // Default implementation does nothing
        let _ = new_messages;
        Ok(())
    }

    /// Get a service from the thread
    fn get_service(&self, service_type: &str) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        let _ = service_type;
        None
    }
}

/// Default implementation of AgentThread that stores messages in memory
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InMemoryThread {
    /// The conversation history
    pub messages: Vec<ChatMessage>,
}

impl InMemoryThread {
    /// Create a new empty thread
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Get all messages
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Add a message to the thread
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Add multiple messages to the thread
    pub fn add_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages.extend(messages);
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[async_trait]
impl AgentThread for InMemoryThread {
    fn serialize(&self) -> Result<Value> {
        Ok(serde_json::to_value(self)?)
    }

    async fn messages_received(&mut self, new_messages: &[ChatMessage]) -> Result<()> {
        self.messages.extend_from_slice(new_messages);
        Ok(())
    }
}
