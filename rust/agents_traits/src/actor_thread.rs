// Copyright (c) Microsoft. All rights reserved.

//! Actor-based thread implementation using Ractor
//!
//! This module provides an actor-based implementation of `AgentThread` using the Ractor library.
//! Each thread runs in its own actor, providing isolation and concurrent message processing.

#[cfg(feature = "actor")]
use crate::{ChatMessage, Result};
#[cfg(feature = "actor")]
use async_trait::async_trait;
#[cfg(feature = "actor")]
use ractor::{Actor, ActorProcessingErr, ActorRef};
#[cfg(feature = "actor")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "actor")]
/// Messages that can be sent to the ThreadActor
pub enum ThreadMessage {
    /// Add messages to the thread
    AddMessages(Vec<ChatMessage>),
    /// Get all messages from the thread
    GetMessages(ractor::RpcReplyPort<Vec<ChatMessage>>),
    /// Clear all messages
    Clear,
    /// Serialize the thread state
    Serialize(ractor::RpcReplyPort<serde_json::Value>),
}

#[cfg(feature = "actor")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadState {
    pub messages: Vec<ChatMessage>,
}

#[cfg(feature = "actor")]
/// Actor implementation for AgentThread
pub struct ThreadActor;

#[cfg(feature = "actor")]
#[async_trait]
impl Actor for ThreadActor {
    type Msg = ThreadMessage;
    type State = ThreadState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _: ActorRef<Self::Msg>,
        _: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        Ok(ThreadState {
            messages: Vec::new(),
        })
    }

    async fn handle(
        &self,
        _: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            ThreadMessage::AddMessages(msgs) => {
                state.messages.extend(msgs);
            }
            ThreadMessage::GetMessages(reply) => {
                let _ = reply.send(state.messages.clone());
            }
            ThreadMessage::Clear => {
                state.messages.clear();
            }
            ThreadMessage::Serialize(reply) => {
                let serialized = serde_json::to_value(&state)
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
                let _ = reply.send(serialized);
            }
        }
        Ok(())
    }
}

#[cfg(feature = "actor")]
/// Actor-based implementation of AgentThread
pub struct ActorThread {
    actor_ref: ActorRef<ThreadMessage>,
}

#[cfg(feature = "actor")]
impl ActorThread {
    /// Create a new actor-based thread
    pub async fn new() -> Result<Self> {
        let (actor_ref, _) = Actor::spawn(None, ThreadActor, ())
            .await
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))?;

        Ok(Self { actor_ref })
    }

    /// Create from serialized state
    pub async fn from_serialized(serialized: serde_json::Value) -> Result<Self> {
        let state: ThreadState = serde_json::from_value(serialized)?;

        let (actor_ref, _) = Actor::spawn(None, ThreadActor, ())
            .await
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))?;

        // Initialize with saved messages
        actor_ref.send_message(ThreadMessage::AddMessages(state.messages))
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))?;

        Ok(Self { actor_ref })
    }

    /// Get all messages
    pub async fn get_messages(&self) -> Result<Vec<ChatMessage>> {
        ractor::call!(self.actor_ref, ThreadMessage::GetMessages)
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))
    }

    /// Add messages
    pub async fn add_messages(&self, messages: Vec<ChatMessage>) -> Result<()> {
        self.actor_ref
            .send_message(ThreadMessage::AddMessages(messages))
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))
    }

    /// Clear messages
    pub async fn clear(&self) -> Result<()> {
        self.actor_ref
            .send_message(ThreadMessage::Clear)
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))
    }
}

#[cfg(feature = "actor")]
#[async_trait]
impl crate::AgentThread for ActorThread {
    fn serialize(&self) -> Result<serde_json::Value> {
        // Note: This would need to be async in a real implementation
        // For now, we'll return an error suggesting to use serialize_async
        Err(crate::AgentError::InvalidOperation(
            "Use actor_thread.serialize_async() for actor-based threads".to_string(),
        ))
    }

    async fn messages_received(&mut self, new_messages: &[ChatMessage]) -> Result<()> {
        self.add_messages(new_messages.to_vec()).await
    }
}

#[cfg(feature = "actor")]
impl ActorThread {
    /// Async version of serialize for actor-based threads
    pub async fn serialize_async(&self) -> Result<serde_json::Value> {
        ractor::call!(self.actor_ref, ThreadMessage::Serialize)
            .map_err(|e| crate::AgentError::ThreadError(e.to_string()))
    }
}

#[cfg(feature = "actor")]
impl Drop for ActorThread {
    fn drop(&mut self) {
        // Gracefully stop the actor when the thread is dropped
        self.actor_ref.stop(None);
    }
}
