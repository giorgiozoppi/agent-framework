// Copyright (c) Microsoft. All rights reserved.

use crate::{
    Edge, EdgeId, ExecutorRegistration, Workflow, WorkflowError, WorkflowResult,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

/// Builder for constructing workflows
pub struct WorkflowBuilder {
    start_executor_id: String,
    name: Option<String>,
    description: Option<String>,
    registrations: HashMap<String, ExecutorRegistration>,
    edges: Vec<Edge>,
    unbound_executors: HashSet<String>,
    output_executors: HashSet<String>,
    edge_counter: AtomicU64,
    conditionless_connections: HashSet<(String, String)>,
}

impl WorkflowBuilder {
    /// Create a new workflow builder with a starting executor
    pub fn new(start_executor: ExecutorIsh) -> WorkflowResult<Self> {
        let mut builder = Self {
            start_executor_id: start_executor.id().to_string(),
            name: None,
            description: None,
            registrations: HashMap::new(),
            edges: Vec::new(),
            unbound_executors: HashSet::new(),
            output_executors: HashSet::new(),
            edge_counter: AtomicU64::new(0),
            conditionless_connections: HashSet::new(),
        };

        builder.track_executor(start_executor)?;
        Ok(builder)
    }

    /// Set the workflow name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the workflow description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Register executors as output sources
    pub fn with_output_from(mut self, executors: &[ExecutorIsh]) -> WorkflowResult<Self> {
        for executor in executors {
            self.track_executor(executor.clone())?;
            self.output_executors.insert(executor.id().to_string());
        }
        Ok(self)
    }

    /// Bind an unbound executor
    pub fn bind_executor(mut self, executor: ExecutorIsh) -> WorkflowResult<Self> {
        let executor_id = executor.id().to_string();

        if !self.unbound_executors.contains(&executor_id) {
            return Err(WorkflowError::invalid_configuration(format!(
                "Executor '{}' is already bound or does not exist",
                executor_id
            )));
        }

        self.track_executor(executor)?;
        self.unbound_executors.remove(&executor_id);
        Ok(self)
    }

    /// Add a direct edge between two executors
    pub fn add_edge(mut self, source: ExecutorIsh, target: ExecutorIsh) -> WorkflowResult<Self> {
        self.add_edge_internal(source, target, None, false)?;
        Ok(self)
    }

    /// Add a direct edge with a condition
    pub fn add_edge_with_condition<F>(
        mut self,
        source: ExecutorIsh,
        target: ExecutorIsh,
        condition: F,
    ) -> WorkflowResult<Self>
    where
        F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
    {
        self.add_edge_internal(source, target, Some(Box::new(condition)), false)?;
        Ok(self)
    }

    /// Add an idempotent edge (won't error if edge already exists)
    pub fn add_edge_idempotent(
        mut self,
        source: ExecutorIsh,
        target: ExecutorIsh,
    ) -> WorkflowResult<Self> {
        self.add_edge_internal(source, target, None, true)?;
        Ok(self)
    }

    fn add_edge_internal(
        &mut self,
        source: ExecutorIsh,
        target: ExecutorIsh,
        condition: Option<Box<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
        idempotent: bool,
    ) -> WorkflowResult<()> {
        self.track_executor(source.clone())?;
        self.track_executor(target.clone())?;

        let source_id = source.id().to_string();
        let target_id = target.id().to_string();
        let connection = (source_id.clone(), target_id.clone());

        if condition.is_none() && self.conditionless_connections.contains(&connection) {
            if idempotent {
                return Ok(());
            }
            return Err(WorkflowError::EdgeAlreadyExists(source_id, target_id));
        }

        let edge_id = EdgeId::new(self.edge_counter.fetch_add(1, Ordering::Relaxed));

        let edge = if let Some(condition) = condition {
            Edge::direct_with_condition(source_id, target_id, edge_id, condition)
        } else {
            self.conditionless_connections.insert(connection);
            Edge::direct(source_id, target_id, edge_id)
        };

        self.edges.push(edge);
        Ok(())
    }

    /// Add a fan-out edge from one source to multiple targets
    pub fn add_fan_out_edge(
        mut self,
        source: ExecutorIsh,
        targets: &[ExecutorIsh],
    ) -> WorkflowResult<Self> {
        self.add_fan_out_edge_internal(source, targets, None)?;
        Ok(self)
    }

    /// Add a fan-out edge with a custom partitioner
    pub fn add_fan_out_edge_with_partitioner<F>(
        mut self,
        source: ExecutorIsh,
        targets: &[ExecutorIsh],
        partitioner: F,
    ) -> WorkflowResult<Self>
    where
        F: Fn(&serde_json::Value, usize) -> Vec<usize> + Send + Sync + 'static,
    {
        self.add_fan_out_edge_internal(source, targets, Some(Box::new(partitioner)))?;
        Ok(self)
    }

    fn add_fan_out_edge_internal(
        &mut self,
        source: ExecutorIsh,
        targets: &[ExecutorIsh],
        partitioner: Option<Box<dyn Fn(&serde_json::Value, usize) -> Vec<usize> + Send + Sync>>,
    ) -> WorkflowResult<()> {
        if targets.is_empty() {
            return Err(WorkflowError::invalid_configuration(
                "Fan-out edge must have at least one target",
            ));
        }

        self.track_executor(source.clone())?;

        let mut target_ids = Vec::new();
        for target in targets {
            self.track_executor(target.clone())?;
            target_ids.push(target.id().to_string());
        }

        let edge_id = EdgeId::new(self.edge_counter.fetch_add(1, Ordering::Relaxed));
        let source_id = source.id().to_string();

        let edge = if let Some(partitioner) = partitioner {
            Edge::fan_out_with_partitioner(source_id, target_ids, edge_id, partitioner)
        } else {
            Edge::fan_out(source_id, target_ids, edge_id)
        };

        self.edges.push(edge);
        Ok(())
    }

    /// Add a fan-in edge from multiple sources to one target
    pub fn add_fan_in_edge(
        mut self,
        target: ExecutorIsh,
        sources: &[ExecutorIsh],
    ) -> WorkflowResult<Self> {
        if sources.is_empty() {
            return Err(WorkflowError::invalid_configuration(
                "Fan-in edge must have at least one source",
            ));
        }

        self.track_executor(target.clone())?;

        let mut source_ids = Vec::new();
        for source in sources {
            self.track_executor(source.clone())?;
            source_ids.push(source.id().to_string());
        }

        let edge_id = EdgeId::new(self.edge_counter.fetch_add(1, Ordering::Relaxed));
        let target_id = target.id().to_string();

        let edge = Edge::fan_in(source_ids, target_id, edge_id);
        self.edges.push(edge);

        Ok(self)
    }

    /// Track an executor in the builder
    fn track_executor(&mut self, executor: ExecutorIsh) -> WorkflowResult<()> {
        let executor_id = executor.id().to_string();

        match executor.kind() {
            ExecutorKind::Unbound => {
                if !self.registrations.contains_key(&executor_id) {
                    self.unbound_executors.insert(executor_id);
                }
            }
            ExecutorKind::Bound(registration) => {
                if let Some(existing) = self.registrations.get(&executor_id) {
                    // Validate that the types match
                    if existing.executor_type != registration.executor_type {
                        return Err(WorkflowError::invalid_configuration(format!(
                            "Executor '{}' type mismatch: {} vs {}",
                            executor_id, existing.executor_type, registration.executor_type
                        )));
                    }
                } else {
                    self.registrations.insert(executor_id.clone(), registration.clone());
                    self.unbound_executors.remove(&executor_id);
                }
            }
        }

        Ok(())
    }

    /// Validate the workflow configuration
    fn validate(&self) -> WorkflowResult<()> {
        if !self.unbound_executors.is_empty() {
            return Err(WorkflowError::invalid_configuration(format!(
                "Unbound executors: {}",
                self.unbound_executors
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        if !self.registrations.contains_key(&self.start_executor_id) {
            return Err(WorkflowError::ExecutorNotFound(self.start_executor_id.clone()));
        }

        Ok(())
    }

    /// Build the workflow
    pub fn build(self) -> WorkflowResult<Workflow> {
        self.validate()?;

        let mut workflow = Workflow::new(self.start_executor_id, self.name, self.description);

        // Add registrations
        workflow.registrations = self.registrations;

        // Add edges to message router
        for edge in self.edges {
            workflow.message_router.add_edge(edge);
        }

        // Set output executors
        workflow.output_executors = self.output_executors;

        Ok(workflow)
    }
}

/// Represents an executor that can be bound or unbound
#[derive(Debug, Clone)]
pub struct ExecutorIsh {
    id: String,
    kind: ExecutorKind,
}

#[derive(Debug, Clone)]
enum ExecutorKind {
    Unbound,
    Bound(ExecutorRegistration),
}

impl ExecutorIsh {
    /// Create an unbound executor placeholder
    pub fn unbound<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            kind: ExecutorKind::Unbound,
        }
    }

    /// Create a bound executor from a registration
    pub fn bound(registration: ExecutorRegistration) -> Self {
        let id = registration.id.clone();
        Self {
            id,
            kind: ExecutorKind::Bound(registration),
        }
    }

    /// Get the executor ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the executor kind
    fn kind(&self) -> &ExecutorKind {
        &self.kind
    }

    /// Check if this is an unbound executor
    pub fn is_unbound(&self) -> bool {
        matches!(self.kind, ExecutorKind::Unbound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentExecutor, FunctionExecutor};

    #[test]
    fn test_workflow_builder_basic() {
        let start_executor = ExecutorIsh::unbound("start");
        let builder = WorkflowBuilder::new(start_executor).unwrap();

        let result = builder.build();
        assert!(result.is_err()); // Should fail because executor is unbound
    }

    #[test]
    fn test_workflow_builder_with_bound_executor() {
        let registration = ExecutorRegistration::new(
            "test".to_string(),
            "TestExecutor".to_string(),
            || {
                Ok(Box::new(FunctionExecutor::new(
                    "test".to_string(),
                    |input: serde_json::Value| Ok(input),
                )))
            },
        );

        let start_executor = ExecutorIsh::bound(registration);
        let builder = WorkflowBuilder::new(start_executor).unwrap();

        let workflow = builder.build().unwrap();
        assert_eq!(workflow.start_executor_id(), "test");
    }

    #[test]
    fn test_add_edge() {
        let reg1 = ExecutorRegistration::new(
            "executor1".to_string(),
            "TestExecutor".to_string(),
            || {
                Ok(Box::new(FunctionExecutor::new(
                    "executor1".to_string(),
                    |input: serde_json::Value| Ok(input),
                )))
            },
        );

        let reg2 = ExecutorRegistration::new(
            "executor2".to_string(),
            "TestExecutor".to_string(),
            || {
                Ok(Box::new(FunctionExecutor::new(
                    "executor2".to_string(),
                    |input: serde_json::Value| Ok(input),
                )))
            },
        );

        let executor1 = ExecutorIsh::bound(reg1);
        let executor2 = ExecutorIsh::bound(reg2);

        let builder = WorkflowBuilder::new(executor1.clone())
            .unwrap()
            .add_edge(executor1, executor2)
            .unwrap();

        let workflow = builder.build().unwrap();
        let edges = workflow.reflect_edges();
        assert!(edges.contains_key("executor1"));
    }
}