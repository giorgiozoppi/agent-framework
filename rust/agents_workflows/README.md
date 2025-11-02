# Agents Workflows

A Rust implementation of the Microsoft Agent Framework workflow orchestration capabilities, ported from the .NET version.

## Overview

This crate provides workflow orchestration capabilities for AI agents in Rust. It allows composing multiple agents into complex workflows with message routing, state management, and conditional execution flows.

## Core Concepts

- **Workflow**: A graph of connected executors that can be executed
- **WorkflowBuilder**: Builder for constructing workflows
- **Executor**: Components that process messages in a workflow
- **WorkflowContext**: Runtime context for executor execution
- **Edge**: Connections between executors with optional conditions

## Key Features

- **Message Routing**: Route messages between executors based on workflow edges
- **State Management**: Persistent state across execution steps with scoped storage
- **Event System**: Comprehensive event system for monitoring workflow execution
- **Conditional Execution**: Support for conditional edges and fan-in/fan-out patterns
- **Agent Integration**: Direct integration with AI agents through the `AgentExecutor`
- **Function Executors**: Simple function-based executors for data processing

## Architecture

The workflow system is built around several key components:

### Executors

Executors are the building blocks of workflows. They process messages and can:
- Send messages to other executors
- Read and write state
- Yield outputs
- Request workflow halt

Types of executors:
- `AgentExecutor`: Wraps an AI agent
- `FunctionExecutor`: Wraps a simple function
- Custom executors implementing the `Executor` trait

### Edges

Edges define how messages flow between executors:
- **Direct edges**: One-to-one connections, optionally with conditions
- **Fan-out edges**: One-to-many connections with optional partitioning
- **Fan-in edges**: Many-to-one connections

### State Management

The workflow provides scoped state management:
- State is organized by scope and key
- Executors have a default scope (their ID)
- State updates are queued and applied at super-step boundaries
- Supports serialization for checkpointing

### Events

Comprehensive event system tracks workflow execution:
- Workflow lifecycle events (started, completed, failed)
- Executor events (invoked, completed, failed)
- State update events
- Output events
- Error and warning events

## Example Usage

```rust
use agents_workflows::{
    ExecutorIsh, ExecutorRegistration, FunctionExecutor, WorkflowBuilder,
};
use serde_json::Value;

// Create a simple function executor
let echo_registration = ExecutorRegistration::new(
    "echo".to_string(),
    "EchoExecutor".to_string(),
    || {
        Ok(Box::new(FunctionExecutor::new(
            "echo".to_string(),
            |input: Value| {
                println!("Echo received: {}", input);
                Ok(format!("Echo: {}", input))
            },
        )))
    },
);

// Build a workflow
let echo_executor = ExecutorIsh::bound(echo_registration);
let workflow = WorkflowBuilder::new(echo_executor.clone())?
    .with_name("Simple Echo Workflow")
    .with_description("A workflow that echoes input")
    .with_output_from(&[echo_executor])?
    .build()?;

println!("Workflow built: {}", workflow.name().unwrap());
```

## Running Examples

```bash
cargo run --example simple_workflow
```

## Implementation Status

This is a Rust port of the .NET Microsoft Agent Framework workflow system. The core functionality has been implemented including:

✅ **Completed:**
- Core workflow types and traits
- Workflow builder with edge support
- State management system
- Event system
- Message routing
- Function and agent executors
- Basic workflow construction

🚧 **In Progress:**
- Workflow execution engine
- Advanced executor features
- Checkpointing system

📋 **Planned:**
- Subworkflow support
- External request handling
- Advanced routing features
- Performance optimizations

## Architecture Notes

This implementation closely follows the .NET version's architecture while adapting to Rust's ownership model and async patterns. Key adaptations include:

- Use of `Arc` and `RwLock` for shared mutable state
- Async traits for executor operations
- Channel-based message passing
- Type-safe JSON handling with `serde_json::Value`

The goal is to provide a familiar API for users of the .NET version while leveraging Rust's safety and performance benefits.

## Dependencies

- `tokio`: Async runtime
- `serde_json`: JSON serialization
- `dashmap`: Concurrent hash maps
- `agents_traits`: Core agent abstractions
- `async-trait`: Async trait support
- `uuid`: Unique identifier generation
- `chrono`: Date/time handling

## License

MIT License - see LICENSE file for details.