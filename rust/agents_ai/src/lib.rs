// Copyright (c) Microsoft. All rights reserved.

//! # Microsoft Agents AI
//!
//! This crate provides concrete implementations of AI agents, including the `ChatClientAgent`
//! which is the primary agent implementation that delegates to an `IChatClient`.
//!
//! ## Core Components
//!
//! - [`ChatClient`]: Trait for chat completion clients
//! - [`ChatClientAgent`]: Agent implementation that uses a ChatClient
//! - [`ChatClientAgentOptions`]: Configuration for ChatClientAgent
//! - [`ChatClientAgentThread`]: Thread implementation for ChatClientAgent
//! - [`ChatMessageStore`]: Trait for storing and retrieving messages

pub mod chat_client;
pub mod chat_client_agent;
pub mod chat_client_agent_options;
pub mod chat_client_agent_thread;
pub mod chat_message_store;

pub use chat_client::{ChatClient, ChatClientMetadata, ChatOptions};
pub use chat_client_agent::ChatClientAgent;
pub use chat_client_agent_options::ChatClientAgentOptions;
pub use chat_client_agent_thread::ChatClientAgentThread;
pub use chat_message_store::{ChatMessageStore, InMemoryChatMessageStore};
