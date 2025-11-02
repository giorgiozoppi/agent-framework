// Copyright (c) Microsoft. All rights reserved.

use crate::{
    chat_client::{ChatClient, ChatOptions},
    chat_client_agent_options::ChatClientAgentOptions,
    chat_client_agent_thread::ChatClientAgentThread,
};
use agents_traits::*;
use async_trait::async_trait;
use std::sync::Arc;

/// Agent implementation that delegates to a ChatClient
pub struct ChatClientAgent {
    /// The underlying chat client
    chat_client: Arc<dyn ChatClient>,
    /// Agent configuration options
    options: ChatClientAgentOptions,
    /// Agent metadata
    metadata: AIAgentMetadata,
}

impl ChatClientAgent {
    /// Create a new ChatClientAgent
    pub fn new(
        chat_client: Arc<dyn ChatClient>,
        options: Option<ChatClientAgentOptions>,
    ) -> Self {
        let options = options.unwrap_or_default();
        
        let id = options.id.clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let metadata = AIAgentMetadata {
            id,
            name: options.name.clone(),
            description: options.description.clone(),
        };

        Self {
            chat_client,
            options,
            metadata,
        }
    }

    /// Create a new agent with simple parameters
    pub fn with_instructions(
        chat_client: Arc<dyn ChatClient>,
        instructions: Option<String>,
        name: Option<String>,
        description: Option<String>,
    ) -> Self {
        let options = ChatClientAgentOptions {
            id: None,
            name,
            instructions,
            description,
            chat_options: None,
            use_provided_chat_client_as_is: false,
        };

        Self::new(chat_client, Some(options))
    }

    /// Get the instructions
    pub fn instructions(&self) -> Option<&str> {
        self.options.instructions.as_deref()
    }

    /// Get the chat options
    pub fn chat_options(&self) -> Option<&ChatOptions> {
        self.options.chat_options.as_ref()
    }

    /// Prepare messages including system instructions
    fn prepare_messages(&self, new_messages: &[ChatMessage], thread_messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // Add system instructions if present
        if let Some(instructions) = &self.options.instructions {
            messages.push(ChatMessage::system(instructions));
        }

        // Add thread history
        messages.extend(thread_messages);

        // Add new messages
        messages.extend_from_slice(new_messages);

        messages
    }

    /// Merge chat options
    fn merge_options(&self, run_options: Option<agent::AgentRunOptions>) -> ChatOptions {
        let options = self.options.chat_options.clone().unwrap_or_default();

        // Merge with run-time options if provided
        if let Some(_run_opts) = run_options {
            // Could merge additional properties here
        }

        options
    }
}

#[async_trait]
impl AIAgent for ChatClientAgent {
    fn metadata(&self) -> &AIAgentMetadata {
        &self.metadata
    }

    fn get_new_thread(&self) -> Box<dyn AgentThread> {
        Box::new(ChatClientAgentThread::new())
    }

    fn deserialize_thread(&self, serialized: serde_json::Value) -> Result<Box<dyn AgentThread>> {
        Ok(Box::new(ChatClientAgentThread::from_serialized(serialized)?))
    }

    async fn run_with_messages(
        &self,
        messages: &[ChatMessage],
        mut thread: Option<&mut dyn AgentThread>,
        options: Option<agent::AgentRunOptions>,
    ) -> Result<AgentRunResponse> {
        // Get thread messages if available
        let thread_messages = if let Some(ref mut thread) = thread {
            // Try to downcast to ChatClientAgentThread to access messages
            // For now, we'll just track new messages
            thread.messages_received(messages).await?;
            Vec::new() // In a full implementation, we'd get history here
        } else {
            Vec::new()
        };

        // Prepare all messages
        let all_messages = self.prepare_messages(messages, thread_messages);

        // Merge options
        let chat_options = self.merge_options(options);

        // Call the chat client
        let mut response = self.chat_client.complete(all_messages, Some(chat_options)).await?;

        // Add response to thread
        if let Some(ref mut thread) = thread {
            thread.messages_received(&response.messages).await?;
        }

        // Set agent ID on response
        response.agent_id = Some(self.id().to_string());

        Ok(response)
    }

    async fn run_streaming(
        &self,
        messages: &[ChatMessage],
        mut thread: Option<&mut dyn AgentThread>,
        options: Option<agent::AgentRunOptions>,
    ) -> Result<agent::AgentResponseStream> {
        // Add messages to thread
        if let Some(ref mut thread) = thread {
            thread.messages_received(messages).await?;
        }

        // Get thread messages
        let thread_messages = Vec::new(); // In full implementation, get from thread

        // Prepare messages
        let all_messages = self.prepare_messages(messages, thread_messages);

        // Merge options
        let chat_options = self.merge_options(options);

        // Call streaming
        self.chat_client.complete_streaming(all_messages, Some(chat_options)).await
    }
}
