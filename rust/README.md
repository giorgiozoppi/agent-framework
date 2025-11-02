# Microsoft Agents AI Framework - Rust Implementation

This directory contains the Rust implementation of the Microsoft Agents AI Framework, providing a high-performance, memory-safe alternative to the .NET and Python implementations.

## Overview

The Rust implementation mirrors the core abstractions and patterns from the .NET version, leveraging Rust's strengths in:
- **Memory Safety**: Zero-cost abstractions without garbage collection
- **Performance**: Native compilation with optimized async/await
- **Concurrency**: Safe, efficient parallel processing
- **Reliability**: Compile-time guarantees for thread safety

## Project Structure

```
rust/
├── microsoft-agents-ai-abstractions/   # Core traits and types
├── microsoft-agents-ai/                # Main agent implementation (planned)
├── microsoft-agents-ai-workflows/      # Workflow orchestration (planned)
├── agent-examples/                     # Example applications
└── Cargo.toml                          # Workspace configuration
```

## Crates

### microsoft-agents-ai-abstractions

The foundational crate providing core traits and types:

- **`AIAgent`**: Base trait for all AI agents
  - Async execution with `run_with_messages()`
  - Streaming support via `run_streaming()`
  - Thread management with `get_new_thread()` and `deserialize_thread()`
  
- **`AgentThread`**: Conversation state management
  - Serialization/deserialization for persistence
  - Message history tracking
  - Extensible service provider pattern

- **`AgentRunResponse`**: Execution results
  - Multiple message support
  - Usage tracking (token consumption)
  - Conversion to streaming updates

- **`ChatMessage`**: Conversation messages
  - Multiple content types (text, images, function calls)
  - Role-based messaging (user, assistant, system, tool)
  - Metadata and timestamps

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
microsoft-agents-ai-abstractions = { path = "../rust/microsoft-agents-ai-abstractions" }
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
```

### Basic Example

```rust
use agents_traits::*;
use async_trait::async_trait;

// Implement a simple echo agent
struct EchoAgent {
    metadata: AIAgentMetadata,
}

#[async_trait]
impl AIAgent for EchoAgent {
    fn metadata(&self) -> &AIAgentMetadata {
        &self.metadata
    }

    fn get_new_thread(&self) -> Box<dyn AgentThread> {
        Box::new(thread::InMemoryThread::new())
    }

    async fn run_with_messages(
        &self,
        messages: &[ChatMessage],
        mut thread: Option<&mut dyn AgentThread>,
        _options: Option<agent::AgentRunOptions>,
    ) -> Result<AgentRunResponse> {
        // Add messages to thread
        if let Some(ref mut thread) = thread {
            thread.messages_received(messages).await?;
        }

        // Generate response
        let response_text = format!("Echo: {}", 
            messages.iter().map(|m| m.text()).collect::<Vec<_>>().join(", "));
        let response = AgentRunResponse::new(ChatMessage::assistant(response_text));

        Ok(response)
    }
    
    // ... other required methods
}

#[tokio::main]
async fn main() -> Result<()> {
    let agent = EchoAgent { 
        metadata: AIAgentMetadata::new("echo-001")
            .with_name("Echo Agent")
    };
    
    let mut thread = agent.get_new_thread();
    let response = agent.run_with_text("Hello!", Some(thread.as_mut()), None).await?;
    
    println!("Agent: {}", response.text());
    Ok(())
}
```

## Running Examples

Run the echo agent example:

```bash
cd rust
cargo run --package agent-examples
```

## Core Concepts

### 1. Agents

Agents are implemented by providing the `AIAgent` trait:

```rust
#[async_trait]
pub trait AIAgent: Send + Sync {
    fn metadata(&self) -> &AIAgentMetadata;
    fn get_new_thread(&self) -> Box<dyn AgentThread>;
    async fn run_with_messages(
        &self,
        messages: &[ChatMessage],
        thread: Option<&mut dyn AgentThread>,
        options: Option<AgentRunOptions>,
    ) -> Result<AgentRunResponse>;
    // ... more methods
}
```

### 2. Conversation Threads

Threads maintain conversation state and can be serialized:

```rust
let thread = agent.get_new_thread();

// Use the thread across multiple turns
agent.run_with_text("Hello", Some(thread.as_mut()), None).await?;
agent.run_with_text("How are you?", Some(thread.as_mut()), None).await?;

// Serialize for persistence
let serialized = thread.serialize()?;
let json = serde_json::to_string(&serialized)?;

// Restore later
let restored = agent.deserialize_thread(serde_json::from_str(&json)?)?;
```

### 3. Messages

Rich message support with multiple content types:

```rust
// Simple text message
let msg = ChatMessage::user("Hello");

// Message with multiple content items
let mut msg = ChatMessage::new(ChatRole::Assistant, "Here's an image:");
msg.add_content(MessageContent::image("https://example.com/image.png"));
```

### 4. Responses

Responses contain messages and metadata:

```rust
let response = agent.run_with_text("Hello", thread, None).await?;

println!("Response: {}", response.text());
println!("Tokens used: {:?}", response.usage);
println!("Agent ID: {:?}", response.agent_id);
```

## Architecture Comparison

### .NET vs Rust

| Feature | .NET | Rust |
|---------|------|------|
| Base Type | `abstract class AIAgent` | `trait AIAgent` |
| Async | `Task<T>` / `IAsyncEnumerable<T>` | `async fn` / `Stream` |
| Thread Safety | Runtime checks | Compile-time guarantees |
| Memory | Garbage collected | RAII with ownership |
| Serialization | `JsonElement` | `serde_json::Value` |
| Error Handling | Exceptions | `Result<T, E>` |

### Key Differences

1. **Ownership Model**: Rust uses ownership and borrowing instead of garbage collection
2. **Trait Objects**: `Box<dyn AgentThread>` instead of interface references
3. **Error Handling**: Explicit `Result` types instead of exceptions
4. **Async Runtime**: Tokio runtime instead of .NET Task scheduler

## Features

### Completed ✅

- Core trait abstractions (`AIAgent`, `AgentThread`, `AgentRunResponse`)
- Message types with role-based communication
- Thread serialization/deserialization
- In-memory thread implementation
- Error handling with `thiserror`
- Basic example application

### Planned 🚧

- **ChatClientAgent**: HTTP-based agent with LLM integration
- **Workflows**: Graph-based orchestration (porting from .NET)
- **Provider Integration**: OpenAI, Azure AI, Anthropic
- **Observability**: Tracing and metrics
- **Checkpointing**: Workflow state persistence
- **Advanced Features**: Function calling, streaming, RAG

## Development

### Building

```bash
# Build all crates
cargo build

# Build specific crate
cargo build --package microsoft-agents-ai-abstractions

# Run tests
cargo test

# Run with release optimizations
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test --package microsoft-agents-ai-abstractions

# Run with output
cargo test -- --nocapture
```

## Contributing

This is a direct port of the .NET implementation. When contributing:

1. Follow Rust idioms and best practices
2. Maintain API parity with .NET where sensible
3. Add tests for new functionality
4. Update documentation

## License

MIT License - See LICENSE file for details

## Related

- [.NET Implementation](../dotnet/)
- [Python Implementation](../python/)
- [Microsoft Agents AI Documentation](https://github.com/microsoft/agent-framework)
