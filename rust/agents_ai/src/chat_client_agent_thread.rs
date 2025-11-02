// Copyright (c) Microsoft. All rights reserved.

use crate::chat_message_store::{ChatMessageStore, InMemoryChatMessageStore};
use agents_traits::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Thread state for serialization
#[derive(Debug, Serialize, Deserialize)]
struct ThreadState {
    conversation_id: Option<String>,
    store_state: Option<serde_json::Value>,
}

/// Thread implementation for ChatClientAgent
pub struct ChatClientAgentThread {
    /// Optional conversation ID for server-managed threads
    conversation_id: Option<String>,
    /// Message store for client-managed threads
    message_store: Option<Box<dyn ChatMessageStore>>,
}

impl ChatClientAgentThread {
    /// Create a new thread
    pub fn new() -> Self {
        Self {
            conversation_id: None,
            message_store: Some(Box::new(InMemoryChatMessageStore::new())),
        }
    }

    /// Create from serialized state
    pub fn from_serialized(serialized: serde_json::Value) -> Result<Self> {
        let state: ThreadState = serde_json::from_value(serialized)?;

        if let Some(conversation_id) = state.conversation_id {
            return Ok(Self {
                conversation_id: Some(conversation_id),
                message_store: None,
            });
        }

        let message_store = if let Some(store_state) = state.store_state {
            Box::new(InMemoryChatMessageStore::from_serialized(store_state)?) as Box<dyn ChatMessageStore>
        } else {
            Box::new(InMemoryChatMessageStore::new()) as Box<dyn ChatMessageStore>
        };

        Ok(Self {
            conversation_id: None,
            message_store: Some(message_store),
        })
    }

    /// Get the conversation ID (for server-managed threads)
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// Set the conversation ID
    pub fn set_conversation_id(&mut self, id: String) -> Result<()> {
        if self.message_store.is_some() {
            return Err(AgentError::InvalidOperation(
                "Cannot set conversation ID when message store is present".to_string(),
            ));
        }
        self.conversation_id = Some(id);
        Ok(())
    }

    /// Get messages from the store (if present)
    pub async fn get_messages(&self) -> Result<Vec<ChatMessage>> {
        if let Some(store) = &self.message_store {
            store.get_messages().await
        } else {
            Ok(Vec::new())
        }
    }

    /// Add messages to the store (if present)
    pub async fn add_messages(&mut self, messages: Vec<ChatMessage>) -> Result<()> {
        if let Some(store) = &mut self.message_store {
            store.add_messages(messages).await
        } else {
            Ok(())
        }
    }
}

impl Default for ChatClientAgentThread {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentThread for ChatClientAgentThread {
    fn serialize(&self) -> Result<serde_json::Value> {
        let store_state = if let Some(store) = &self.message_store {
            Some(store.serialize()?)
        } else {
            None
        };

        let state = ThreadState {
            conversation_id: self.conversation_id.clone(),
            store_state,
        };

        Ok(serde_json::to_value(state)?)
    }

    async fn messages_received(&mut self, new_messages: &[ChatMessage]) -> Result<()> {
        if let Some(store) = &mut self.message_store {
            store.add_messages(new_messages.to_vec()).await
        } else {
            Ok(())
        }
    }
}
