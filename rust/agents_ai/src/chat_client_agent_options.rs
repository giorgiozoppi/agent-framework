
use crate::chat_client::ChatOptions;
use serde::{Deserialize, Serialize};

/// Configuration options for ChatClientAgent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatClientAgentOptions {
    /// Unique identifier for the agent
    pub id: Option<String>,

    /// Human-readable name for the agent
    pub name: Option<String>,

    /// System instructions that guide the agent's behavior
    pub instructions: Option<String>,

    /// Description of the agent's purpose and capabilities
    pub description: Option<String>,

    /// Default chat options to use
    pub chat_options: Option<ChatOptions>,

    /// Whether to use the provided chat client as-is without decorators
    pub use_provided_chat_client_as_is: bool,
}

impl ChatClientAgentOptions {
    /// Create new options
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            instructions: None,
            description: None,
            chat_options: None,
            use_provided_chat_client_as_is: false,
        }
    }

    /// Set the agent ID
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the agent name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the instructions
    pub fn with_instructions<S: Into<String>>(mut self, instructions: S) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Set the description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set chat options
    pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
        self.chat_options = Some(options);
        self
    }

    /// Clone the options
    pub fn clone_options(&self) -> Self {
        self.clone()
    }
}

impl Default for ChatClientAgentOptions {
    fn default() -> Self {
        Self::new()
    }
}
