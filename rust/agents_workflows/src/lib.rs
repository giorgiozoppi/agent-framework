// Copyright (c) Microsoft. All rights reserved.

//! # Agents Workflows
//!
//! This crate provides workflow orchestration capabilities for AI agents in Rust.
//! It allows composing multiple agents into complex workflows with message routing,
//! state management, and conditional execution flows.
//!
//! ## Core Concepts
//!
//! - [`Workflow`]: A graph of connected executors that can be executed
//! - [`WorkflowBuilder`]: Builder for constructing workflows
//! - [`Executor`]: Components that process messages in a workflow
//! - [`WorkflowContext`]: Runtime context for executor execution
//! - [`Edge`]: Connections between executors with optional conditions

pub mod context;
pub mod edge;
pub mod error;
pub mod events;
// pub mod execution;
pub mod executor;
pub mod message_router;
pub mod run;
pub mod state;
pub mod workflow;
pub mod workflow_builder;

pub use context::WorkflowContext;
pub use edge::{Edge, EdgeData, EdgeId, EdgeInfo};
pub use error::{WorkflowError, WorkflowResult};
pub use events::{WorkflowEvent, WorkflowEventKind};
// pub use execution::WorkflowExecutor;
pub use executor::{Executor, ExecutorOptions, ExecutorRegistration, FunctionExecutor, ProtocolDescriptor};
pub use message_router::MessageRouter;
pub use run::{Run, RunControlHandle, RunStatus};
pub use state::StateManager;
pub use workflow::Workflow;
pub use workflow_builder::{WorkflowBuilder, ExecutorIsh};
