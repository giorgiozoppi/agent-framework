// Copyright (c) Microsoft. All rights reserved.

use serde::{Deserialize, Serialize};

/// Details about token usage for an agent run
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageDetails {
    /// Number of input tokens consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,

    /// Number of output tokens generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,

    /// Total tokens (input + output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,

    /// Additional usage metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl UsageDetails {
    /// Create new usage details
    pub fn new(input_tokens: Option<u32>, output_tokens: Option<u32>) -> Self {
        let total_tokens = match (input_tokens, output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            (Some(i), None) => Some(i),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        };

        Self {
            input_tokens,
            output_tokens,
            total_tokens,
            additional_properties: None,
        }
    }
}
