// Copyright (c) Microsoft. All rights reserved.

use crate::{
    context::{OutgoingMessage, WorkflowContextImpl},
    Executor, Run, RunControlHandle, RunStatus, StateManager, Workflow,
    WorkflowEvent, WorkflowEventKind, WorkflowResult,
};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Executes workflows
pub struct WorkflowExecutor {
    workflow: Arc<Workflow>,
    state_manager: StateManager,
    executor_instances: Arc<DashMap<String, Arc<RwLock<Box<dyn Executor>>>>>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow: Arc::new(workflow),
            state_manager: StateManager::new(),
            executor_instances: Arc::new(DashMap::new()),
        }
    }

    /// Start a new workflow run
    pub async fn start_run(&self, run_id: Option<String>) -> WorkflowResult<Run> {
        let run_id = run_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        // Create communication channels
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
        let (cancel_sender, mut cancel_receiver) = mpsc::unbounded_channel();
        let (completion_sender, completion_receiver) = mpsc::unbounded_channel();

        // Create control handle
        let control_handle = RunControlHandle::new(response_sender, cancel_sender, completion_receiver);

        // Create run instance
        let run = Run::new(Some(run_id.clone()), event_receiver, control_handle);

        // Send workflow started event
        let start_event = WorkflowEvent::new(WorkflowEventKind::WorkflowStarted {
            workflow_id: self.workflow.start_executor_id().to_string(),
            run_id: run_id.clone(),
        });
        event_sender.send(start_event).ok();

        // Initialize executor instances
        self.initialize_executors().await?;

        // Start execution task
        let workflow = Arc::clone(&self.workflow);
        let state_manager = self.state_manager.clone();
        let executor_instances = Arc::clone(&self.executor_instances);
        let event_sender_clone = event_sender.clone();

        tokio::spawn(async move {
            let result = Self::run_workflow_loop(
                workflow,
                state_manager,
                executor_instances,
                event_sender_clone,
                &mut response_receiver,
                &mut cancel_receiver,
                run_id.clone(),
            )
            .await;

            let final_status = match result {
                Ok(_) => RunStatus::Completed,
                Err(e) => {
                    error!("Workflow execution failed: {}", e);
                    RunStatus::Failed
                }
            };

            // Send completion event
            let completion_event = if final_status == RunStatus::Completed {
                WorkflowEvent::new(WorkflowEventKind::WorkflowCompleted {
                    workflow_id: self.workflow.start_executor_id().to_string(),
                    run_id: run_id.clone(),
                })
            } else {
                WorkflowEvent::new(WorkflowEventKind::WorkflowFailed {
                    workflow_id: self.workflow.start_executor_id().to_string(),
                    run_id: run_id.clone(),
                    error: "Execution failed".to_string(),
                })
            };

            event_sender.send(completion_event).ok();
            completion_sender.send(final_status).ok();
        });

        run.set_status(RunStatus::Running).await;
        Ok(run)
    }

    /// Initialize all executor instances
    async fn initialize_executors(&self) -> WorkflowResult<()> {
        for (executor_id, registration) in &self.workflow.registrations {
            let executor_instance = registration.create_instance().await?;
            self.executor_instances.insert(
                executor_id.clone(),
                Arc::new(RwLock::new(executor_instance)),
            );
        }
        Ok(())
    }

    /// Main workflow execution loop
    async fn run_workflow_loop(
        workflow: Arc<Workflow>,
        state_manager: StateManager,
        executor_instances: Arc<DashMap<String, Arc<RwLock<Box<dyn Executor>>>>>,
        event_sender: mpsc::UnboundedSender<WorkflowEvent>,
        response_receiver: &mut mpsc::UnboundedReceiver<Value>,
        cancel_receiver: &mut mpsc::UnboundedReceiver<()>,
        _run_id: String,
    ) -> WorkflowResult<()> {
        let halt_requested = Arc::new(AtomicBool::new(false));
        let mut super_step_counter = 0u64;

        // Create channels for internal message passing
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        let (output_sender, mut output_receiver) = mpsc::unbounded_channel();

        // Initialize all executors
        for entry in executor_instances.iter() {
            let executor_id = entry.key();
            let executor_instance = entry.value();
            let context = WorkflowContextImpl::new(
                executor_id.clone(),
                state_manager.clone(),
                event_sender.clone(),
                message_sender.clone(),
                output_sender.clone(),
                Arc::clone(&halt_requested),
            );

            let mut executor = executor_instance.write().await;
            executor.initialize(&context).await?;
            info!("Initialized executor: {}", executor_id);
        }

        // Start with initial message to start executor
        let start_message = OutgoingMessage {
            message: Value::Null, // Empty initial message
            target_id: Some(workflow.start_executor_id().to_string()),
            source_id: "system".to_string(),
        };
        message_sender.send(start_message).ok();

        loop {
            super_step_counter += 1;
            let step_id = format!("step_{}", super_step_counter);

            debug!("Starting super step: {}", step_id);

            // Send super step started event
            let step_start_event = WorkflowEvent::new(WorkflowEventKind::SuperStepStarted {
                step_id: step_id.clone(),
            });
            event_sender.send(step_start_event).ok();

            // Process all available messages in this super step
            let mut messages_processed = 0;
            let mut active_executors = Vec::new();

            // Collect all available messages
            let mut pending_messages = Vec::new();
            while let Ok(message) = message_receiver.try_recv() {
                pending_messages.push(message);
            }

            // Process each message
            for message in pending_messages {
                if let Some(target_id) = &message.target_id {
                    if let Some(executor_instance) = executor_instances.get(target_id) {
                        let context = WorkflowContextImpl::new(
                            target_id.clone(),
                            state_manager.clone(),
                            event_sender.clone(),
                            message_sender.clone(),
                            output_sender.clone(),
                            Arc::clone(&halt_requested),
                        );

                        // Send executor invoked event
                        let invoke_event = WorkflowEvent::new(WorkflowEventKind::ExecutorInvoked {
                            executor_id: target_id.clone(),
                            message_type: "unknown".to_string(), // Could be improved with type info
                        });
                        event_sender.send(invoke_event).ok();

                        let mut executor = executor_instance.write().await;
                        match executor.process_message(message.message.clone(), &context).await {
                            Ok(_) => {
                                let complete_event = WorkflowEvent::new(WorkflowEventKind::ExecutorCompleted {
                                    executor_id: target_id.clone(),
                                    success: true,
                                });
                                event_sender.send(complete_event).ok();
                                active_executors.push(target_id.clone());
                                messages_processed += 1;
                            }
                            Err(e) => {
                                let error_event = WorkflowEvent::new(WorkflowEventKind::ExecutorFailed {
                                    executor_id: target_id.clone(),
                                    error: e.to_string(),
                                });
                                event_sender.send(error_event).ok();
                                error!("Executor {} failed: {}", target_id, e);
                            }
                        }
                    } else {
                        warn!("Target executor not found: {}", target_id);
                    }
                }

                // Route the message to connected executors
                if let Ok(targets) = workflow.message_router.route_message(
                    &message.source_id,
                    &message.message,
                    message.target_id.as_deref(),
                ) {
                    for target in targets {
                        let routed_message = OutgoingMessage {
                            message: target.message,
                            target_id: Some(target.executor_id),
                            source_id: message.source_id.clone(),
                        };
                        message_sender.send(routed_message).ok();
                    }
                }
            }

            // Process any outputs
            while let Ok(output) = output_receiver.try_recv() {
                let output_event = WorkflowEvent::new(WorkflowEventKind::WorkflowOutput {
                    executor_id: "unknown".to_string(), // Could be improved with source tracking
                    output,
                });
                event_sender.send(output_event).ok();
            }

            // Apply state updates
            state_manager.apply_queued_updates().await?;

            // Send super step completed event
            let step_complete_event = WorkflowEvent::new(WorkflowEventKind::SuperStepCompleted {
                step_id: step_id.clone(),
                message_count: messages_processed,
            });
            event_sender.send(step_complete_event).ok();

            debug!(
                "Completed super step: {} (processed {} messages)",
                step_id, messages_processed
            );

            // Check for halt conditions
            if halt_requested.load(Ordering::Relaxed) {
                info!("Workflow halted by request");
                break;
            }

            // Check for cancellation
            if cancel_receiver.try_recv().is_ok() {
                info!("Workflow cancelled");
                break;
            }

            // If no messages were processed and no external input pending, workflow is complete
            if messages_processed == 0 && response_receiver.try_recv().is_err() {
                info!("No more messages to process, workflow complete");
                break;
            }

            // Handle external responses
            if let Ok(response) = response_receiver.try_recv() {
                // Convert response to message and send to appropriate executor
                // This is a simplified implementation
                let response_message = OutgoingMessage {
                    message: response,
                    target_id: Some(workflow.start_executor_id().to_string()),
                    source_id: "external".to_string(),
                };
                message_sender.send(response_message).ok();
            }

            // Small delay to prevent busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        info!("Workflow execution completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutorRegistration, FunctionExecutor, WorkflowBuilder, ExecutorIsh};

    #[tokio::test]
    async fn test_simple_workflow_execution() {
        // Create a simple function executor
        let registration = ExecutorRegistration::new(
            "echo".to_string(),
            "EchoExecutor".to_string(),
            || {
                Ok(Box::new(FunctionExecutor::new(
                    "echo".to_string(),
                    |input: Value| Ok(input),
                )))
            },
        );

        let start_executor = ExecutorIsh::bound(registration);
        let workflow = WorkflowBuilder::new(start_executor)
            .unwrap()
            .with_name("Echo Workflow")
            .build()
            .unwrap();

        let executor = WorkflowExecutor::new(workflow);
        let mut run = executor.start_run(None).await.unwrap();

        // Wait a bit for execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let status = run.get_status().await;
        println!("Run status: {:?}", status);

        let events = run.get_events().await;
        println!("Events: {:?}", events);

        assert!(events.len() > 0);
    }
}