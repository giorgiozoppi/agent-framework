// Copyright (c) Microsoft. All rights reserved.

use agents_traits::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for storing and retrieving chat messages
#[async_trait]
pub trait ChatMessageStore: Send + Sync {
    /// Add messages to the store
    async fn add_messages(&mut self, messages: Vec<ChatMessage>) -> Result<()>;

    /// Get all messages from the store
    async fn get_messages(&self) -> Result<Vec<ChatMessage>>;

    /// Clear all messages from the store
    async fn clear(&mut self) -> Result<()>;

    /// Serialize the store state
    fn serialize(&self) -> Result<serde_json::Value>;
}

/// In-memory implementation of ChatMessageStore
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InMemoryChatMessageStore {
    messages: Vec<ChatMessage>,
}

impl InMemoryChatMessageStore {
    /// Create a new empty message store
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Create from serialized state
    pub fn from_serialized(serialized: serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(serialized)?)
    }

    /// Get the messages
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }
}

#[async_trait]
impl ChatMessageStore for InMemoryChatMessageStore {
    async fn add_messages(&mut self, messages: Vec<ChatMessage>) -> Result<()> {
        self.messages.extend(messages);
        Ok(())
    }

    async fn get_messages(&self) -> Result<Vec<ChatMessage>> {
        Ok(self.messages.clone())
    }

    async fn clear(&mut self) -> Result<()> {
        self.messages.clear();
        Ok(())
    }

    fn serialize(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}
