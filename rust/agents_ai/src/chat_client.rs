// Copyright (c) Microsoft. All rights reserved.

use agents_traits::*;
use async_trait::async_trait;
use std::collections::HashMap;

/// Metadata about a chat client
#[derive(Debug, Clone)]
pub struct ChatClientMetadata {
    /// The provider name (e.g., "openai", "azure-openai", "anthropic")
    pub provider_name: Option<String>,
    /// The model identifier
    pub model_id: Option<String>,
    /// Additional metadata
    pub additional_properties: Option<HashMap<String, serde_json::Value>>,
}

impl ChatClientMetadata {
    /// Create new chat client metadata
    pub fn new() -> Self {
        Self {
            provider_name: None,
            model_id: None,
            additional_properties: None,
        }
    }

    /// Set the provider name
    pub fn with_provider_name<S: Into<String>>(mut self, name: S) -> Self {
        self.provider_name = Some(name.into());
        self
    }

    /// Set the model ID
    pub fn with_model_id<S: Into<String>>(mut self, model_id: S) -> Self {
        self.model_id = Some(model_id.into());
        self
    }
}

impl Default for ChatClientMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for chat completion
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChatOptions {
    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    pub top_p: Option<f32>,
    /// Stop sequences
    pub stop_sequences: Option<Vec<String>>,
    /// Tools/functions available to the model
    pub tools: Option<Vec<serde_json::Value>>,
    /// Tool choice strategy
    pub tool_choice: Option<String>,
    /// Additional provider-specific options
    pub additional_properties: Option<HashMap<String, serde_json::Value>>,
}

impl ChatOptions {
    /// Create new chat options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Clone the options
    pub fn clone_options(&self) -> Self {
        self.clone()
    }
}

/// Trait for chat completion clients (equivalent to IChatClient in .NET)
#[async_trait]
pub trait ChatClient: Send + Sync {
    /// Get metadata about this chat client
    fn metadata(&self) -> &ChatClientMetadata;

    /// Complete a chat conversation
    async fn complete(
        &self,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Result<AgentRunResponse>;

    /// Complete a chat conversation with streaming
    async fn complete_streaming(
        &self,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Result<agent::AgentResponseStream>;
}
