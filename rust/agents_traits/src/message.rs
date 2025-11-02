// Copyright (c) Microsoft. All rights reserved.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents the role of a participant in a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// A system message (instructions, context)
    System,
    /// A message from the user
    User,
    /// A message from the AI assistant
    Assistant,
    /// A tool/function call or response
    Tool,
}

/// Different types of content that can be in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    /// Plain text content
    Text {
        text: String,
    },
    /// Image content
    Image {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Function/tool call
    FunctionCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// Function/tool result
    FunctionResult {
        call_id: String,
        result: String,
    },
    /// Usage information
    Usage {
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        total_tokens: Option<u32>,
    },
}

impl MessageContent {
    /// Create a text content
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text { text: text.into() }
    }

    /// Create an image content
    pub fn image<S: Into<String>>(url: S) -> Self {
        MessageContent::Image {
            url: url.into(),
            detail: None,
        }
    }

    /// Extract text from content if it's a text type
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// A single message in a chat conversation
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// The role of the message sender
    pub role: ChatRole,

    /// The name of the author (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,

    /// The content items of the message
    pub contents: Vec<MessageContent>,

    /// When the message was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    /// Additional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<HashMap<String, serde_json::Value>>,
}

impl Clone for ChatMessage {
    fn clone(&self) -> Self {
        Self {
            message_id: self.message_id.clone(),
            role: self.role,
            author_name: self.author_name.clone(),
            contents: self.contents.clone(),
            created_at: self.created_at,
            additional_properties: self.additional_properties.clone(),
        }
    }
}

impl ChatMessage {
    /// Create a new message with the given role and text content
    pub fn new<S: Into<String>>(role: ChatRole, text: S) -> Self {
        Self {
            message_id: Some(Uuid::new_v4().to_string()),
            role,
            author_name: None,
            contents: vec![MessageContent::text(text)],
            created_at: Some(Utc::now()),
            additional_properties: None,
        }
    }

    /// Create a user message
    pub fn user<S: Into<String>>(text: S) -> Self {
        Self::new(ChatRole::User, text)
    }

    /// Create an assistant message
    pub fn assistant<S: Into<String>>(text: S) -> Self {
        Self::new(ChatRole::Assistant, text)
    }

    /// Create a system message
    pub fn system<S: Into<String>>(text: S) -> Self {
        Self::new(ChatRole::System, text)
    }

    /// Create a message with the given role and contents
    pub fn with_contents(role: ChatRole, contents: Vec<MessageContent>) -> Self {
        Self {
            message_id: Some(Uuid::new_v4().to_string()),
            role,
            author_name: None,
            contents,
            created_at: Some(Utc::now()),
            additional_properties: None,
        }
    }

    /// Get the concatenated text from all text content items
    pub fn text(&self) -> String {
        self.contents
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Add a content item to this message
    pub fn add_content(&mut self, content: MessageContent) {
        self.contents.push(content);
    }
}
