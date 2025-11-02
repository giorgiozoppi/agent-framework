// Copyright (c) Microsoft. All rights reserved.

use crate::{WorkflowContext, WorkflowResult};
use agents_traits::AIAgent;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A component that processes messages in a workflow
#[async_trait]
pub trait Executor: Send + Sync {
    /// Unique identifier for the executor
    fn id(&self) -> &str;

    /// Get executor options
    fn options(&self) -> &ExecutorOptions;

    /// Whether this executor supports cross-run sharing
    fn is_cross_run_shareable(&self) -> bool {
        false
    }

    /// Initialize the executor (called once per workflow run)
    async fn initialize(&mut self, context: &dyn WorkflowContext) -> WorkflowResult<()> {
        let _ = context;
        Ok(())
    }

    /// Process an incoming message
    async fn process_message(
        &mut self,
        message: Value,
        context: &dyn WorkflowContext,
    ) -> WorkflowResult<()>;

    /// Reset the executor state (for resettable executors)
    async fn reset(&mut self) -> WorkflowResult<bool> {
        Ok(true) // By default, indicates no reset needed
    }

    /// Get the types of messages this executor can send
    fn sent_types(&self) -> HashSet<String> {
        let mut types = HashSet::new();
        types.insert("object".to_string());
        types
    }

    /// Get the types of messages this executor can yield as output
    fn yield_types(&self) -> HashSet<String> {
        HashSet::new()
    }

    /// Get protocol information for this executor
    fn describe_protocol(&self) -> ProtocolDescriptor {
        ProtocolDescriptor {
            input_types: vec!["object".to_string()],
            output_types: self.yield_types().into_iter().collect(),
        }
    }

    /// Get a service from this executor
    fn get_service(&self, service_type: &str) -> Option<Box<dyn Any + Send + Sync>> {
        let _ = service_type;
        None
    }
}

/// Configuration options for an executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorOptions {
    /// Whether to automatically yield handler return values as output
    pub auto_yield_output_handler_result: bool,
    /// Custom configuration
    pub custom_config: Option<Value>,
}

impl Default for ExecutorOptions {
    fn default() -> Self {
        Self {
            auto_yield_output_handler_result: true,
            custom_config: None,
        }
    }
}

/// Protocol descriptor for an executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolDescriptor {
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

/// Registration information for an executor
#[derive(Debug, Clone)]
pub struct ExecutorRegistration {
    pub id: String,
    pub executor_type: String,
    pub factory: ExecutorFactory,
    pub supports_concurrent: bool,
    pub supports_resetting: bool,
}

impl ExecutorRegistration {
    pub fn new<F>(id: String, executor_type: String, factory: F) -> Self
    where
        F: Fn() -> WorkflowResult<Box<dyn Executor>> + Send + Sync + 'static,
    {
        Self {
            id,
            executor_type,
            factory: ExecutorFactory::new(factory),
            supports_concurrent: false,
            supports_resetting: true,
        }
    }

    pub fn with_concurrent_support(mut self, supports: bool) -> Self {
        self.supports_concurrent = supports;
        self
    }

    pub fn with_reset_support(mut self, supports: bool) -> Self {
        self.supports_resetting = supports;
        self
    }

    pub async fn create_instance(&self) -> WorkflowResult<Box<dyn Executor>> {
        (self.factory.create)()
    }

    pub async fn try_reset(&self, executor: &mut dyn Executor) -> WorkflowResult<bool> {
        if self.supports_resetting {
            executor.reset().await
        } else {
            Ok(true)
        }
    }
}

/// Factory for creating executor instances
pub struct ExecutorFactory {
    create: Arc<dyn Fn() -> WorkflowResult<Box<dyn Executor>> + Send + Sync>,
}

impl ExecutorFactory {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn() -> WorkflowResult<Box<dyn Executor>> + Send + Sync + 'static,
    {
        Self {
            create: Arc::new(factory),
        }
    }
}

impl Clone for ExecutorFactory {
    fn clone(&self) -> Self {
        Self {
            create: Arc::clone(&self.create),
        }
    }
}

impl std::fmt::Debug for ExecutorFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutorFactory").finish()
    }
}

/// Agent-based executor that wraps an AIAgent
pub struct AgentExecutor<A: AIAgent> {
    id: String,
    agent: Arc<RwLock<A>>,
    thread: Arc<RwLock<Option<Box<dyn agents_traits::AgentThread>>>>,
    options: ExecutorOptions,
}

impl<A: AIAgent> AgentExecutor<A> {
    pub fn new(id: String, agent: A) -> Self {
        Self {
            id,
            agent: Arc::new(RwLock::new(agent)),
            thread: Arc::new(RwLock::new(None)),
            options: ExecutorOptions::default(),
        }
    }

    pub fn with_options(mut self, options: ExecutorOptions) -> Self {
        self.options = options;
        self
    }
}

#[async_trait]
impl<A: AIAgent + 'static> Executor for AgentExecutor<A> {
    fn id(&self) -> &str {
        &self.id
    }

    fn options(&self) -> &ExecutorOptions {
        &self.options
    }

    async fn initialize(&mut self, _context: &dyn WorkflowContext) -> WorkflowResult<()> {
        let agent = self.agent.read().await;
        let new_thread = agent.get_new_thread();
        *self.thread.write().await = Some(new_thread);
        Ok(())
    }

    async fn process_message(
        &mut self,
        message: Value,
        context: &dyn WorkflowContext,
    ) -> WorkflowResult<()> {
        // Convert JSON message to chat message if it's a string
        let chat_message = if let Value::String(text) = message {
            agents_traits::ChatMessage::user(text)
        } else {
            // Try to deserialize as ChatMessage
            serde_json::from_value(message)?
        };

        let agent = self.agent.read().await;
        let mut thread_guard = self.thread.write().await;

        if let Some(ref mut thread) = *thread_guard {
            let response = agent
                .run_with_message(chat_message, Some(thread.as_mut()), None)
                .await?;

            // Yield the response as output if auto-yield is enabled
            if self.options.auto_yield_output_handler_result {
                let output_value = serde_json::to_value(&response)?;
                context.yield_output(output_value).await?;
            }
        }

        Ok(())
    }

    fn yield_types(&self) -> HashSet<String> {
        if self.options.auto_yield_output_handler_result {
            let mut types = HashSet::new();
            types.insert("AgentRunResponse".to_string());
            types
        } else {
            HashSet::new()
        }
    }
}

/// Function-based executor for simple processing
pub struct FunctionExecutor<T, R> {
    id: String,
    function: Arc<dyn Fn(T) -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    options: ExecutorOptions,
    _phantom: std::marker::PhantomData<(T, R)>,
}

impl<T, R> FunctionExecutor<T, R>
where
    T: for<'de> serde::Deserialize<'de> + Send + 'static,
    R: serde::Serialize + Send + 'static,
{
    pub fn new<F>(id: String, function: F) -> Self
    where
        F: Fn(T) -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
    {
        Self {
            id,
            function: Arc::new(function),
            options: ExecutorOptions::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_options(mut self, options: ExecutorOptions) -> Self {
        self.options = options;
        self
    }
}

#[async_trait]
impl<T, R> Executor for FunctionExecutor<T, R>
where
    T: for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
    R: serde::Serialize + Send + Sync + 'static,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn options(&self) -> &ExecutorOptions {
        &self.options
    }

    async fn process_message(
        &mut self,
        message: Value,
        context: &dyn WorkflowContext,
    ) -> WorkflowResult<()> {
        let input: T = serde_json::from_value(message)
            .map_err(|e| crate::WorkflowError::async_error(format!("Failed to deserialize input: {}", e)))?;

        let result = (self.function)(input)
            .map_err(|e| crate::WorkflowError::async_error(format!("Function execution failed: {}", e)))?;

        if self.options.auto_yield_output_handler_result {
            let output_value = serde_json::to_value(&result)?;
            context.yield_output(output_value).await?;
        }

        Ok(())
    }

    fn yield_types(&self) -> HashSet<String> {
        if self.options.auto_yield_output_handler_result {
            let mut types = HashSet::new();
            types.insert(std::any::type_name::<R>().to_string());
            types
        } else {
            HashSet::new()
        }
    }
}