// Copyright (c) Microsoft. All rights reserved.

//! # Microsoft Agents AI Abstractions
//!
//! This crate provides the foundational abstractions for building AI agents in Rust.
//! It mirrors the .NET implementation of the Microsoft Agent Framework.
//!
//! ## Core Concepts
//!
//! - [`AIAgent`]: The base trait for all AI agents
//! - [`AgentThread`]: Represents the conversation state for an agent
//! - [`AgentRunResponse`]: The response from an agent execution
//! - [`ChatMessage`]: Individual messages in a conversation

pub mod agent;
pub mod error;
pub mod message;
pub mod response;
pub mod thread;
pub mod usage;

// Optional actor-based thread implementation
#[cfg(feature = "actor")]
pub mod actor_thread;

pub use agent::{AIAgent, AIAgentMetadata};
pub use error::{AgentError, Result};
pub use message::{ChatMessage, ChatRole, MessageContent};
pub use response::{AgentRunResponse, AgentRunResponseUpdate};
pub use thread::AgentThread;
pub use usage::UsageDetails;

#[cfg(feature = "actor")]
pub use actor_thread::ActorThread;
