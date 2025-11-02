// Copyright (c) Microsoft. All rights reserved.

use crate::{
    EdgeInfo, ExecutorRegistration, MessageRouter, ProtocolDescriptor,
    WorkflowError, WorkflowResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A workflow that can be executed
#[derive(Debug)]
pub struct Workflow {
    pub(crate) start_executor_id: String,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) registrations: HashMap<String, ExecutorRegistration>,
    pub(crate) message_router: MessageRouter,
    pub(crate) output_executors: std::collections::HashSet<String>,
    ownership_token: Arc<Mutex<Option<OwnershipToken>>>,
    needs_reset: Arc<std::sync::atomic::AtomicBool>,
}

impl Workflow {
    pub(crate) fn new(
        start_executor_id: String,
        name: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            start_executor_id,
            name,
            description,
            registrations: HashMap::new(),
            message_router: MessageRouter::new(),
            output_executors: std::collections::HashSet::new(),
            ownership_token: Arc::new(Mutex::new(None)),
            needs_reset: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get the starting executor ID
    pub fn start_executor_id(&self) -> &str {
        &self.start_executor_id
    }

    /// Get the workflow name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the workflow description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Check if all executors support concurrent execution
    pub fn allow_concurrent(&self) -> bool {
        self.registrations
            .values()
            .all(|reg| reg.supports_concurrent)
    }

    /// Get executor IDs that don't support concurrent execution
    pub fn non_concurrent_executor_ids(&self) -> Vec<&str> {
        self.registrations
            .values()
            .filter(|reg| !reg.supports_concurrent)
            .map(|reg| reg.id.as_str())
            .collect()
    }

    /// Get reflection information about edges
    pub fn reflect_edges(&self) -> HashMap<String, Vec<EdgeInfo>> {
        let mut edge_info = HashMap::new();

        for (source_id, _) in &self.registrations {
            let edges = self.message_router.get_edges_from(source_id);
            let infos: Vec<EdgeInfo> = edges.iter().map(|e| EdgeInfo::from(*e)).collect();
            if !infos.is_empty() {
                edge_info.insert(source_id.clone(), infos);
            }
        }

        edge_info
    }

    /// Get information about external request ports
    pub fn reflect_ports(&self) -> HashMap<String, RequestPortInfo> {
        // For now, return empty as we haven't implemented request ports yet
        HashMap::new()
    }

    /// Take ownership of the workflow for execution
    pub fn take_ownership(&self, token: OwnershipToken) -> WorkflowResult<()> {
        let mut current_token = self.ownership_token.lock().unwrap();

        if let Some(existing) = &*current_token {
            if existing.owner_id != token.owner_id {
                return Err(WorkflowError::ownership_error(format!(
                    "Workflow is already owned by '{}'",
                    existing.owner_id
                )));
            }
        }

        if self.needs_reset.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(WorkflowError::ownership_error(
                "Workflow needs reset before reuse",
            ));
        }

        *current_token = Some(token);
        Ok(())
    }

    /// Release ownership of the workflow
    pub async fn release_ownership(&self, token: &OwnershipToken) -> WorkflowResult<()> {
        let mut current_token = self.ownership_token.lock().unwrap();

        match &*current_token {
            Some(current) if current.owner_id == token.owner_id => {
                *current_token = None;
                drop(current_token);

                // Try to reset all resettable executors
                self.try_reset_executors().await?;
            }
            Some(current) => {
                return Err(WorkflowError::ownership_error(format!(
                    "Cannot release ownership: owned by '{}', not '{}'",
                    current.owner_id, token.owner_id
                )));
            }
            None => {
                return Err(WorkflowError::ownership_error(
                    "Workflow is not currently owned",
                ));
            }
        }

        Ok(())
    }

    /// Check ownership with optional token
    pub fn check_ownership(&self, expected_token: Option<&OwnershipToken>) -> WorkflowResult<()> {
        let current_token = self.ownership_token.lock().unwrap();

        match (&*current_token, expected_token) {
            (Some(current), Some(expected)) if current.owner_id == expected.owner_id => Ok(()),
            (Some(current), Some(expected)) => Err(WorkflowError::ownership_error(format!(
                "Ownership mismatch: expected '{}', actual '{}'",
                expected.owner_id, current.owner_id
            ))),
            (Some(current), None) => Err(WorkflowError::ownership_error(format!(
                "Workflow is owned by '{}' but no ownership expected",
                current.owner_id
            ))),
            (None, Some(expected)) => Err(WorkflowError::ownership_error(format!(
                "Expected ownership by '{}' but workflow is unowned",
                expected.owner_id
            ))),
            (None, None) => Ok(()),
        }
    }

    /// Try to reset all executor registrations
    async fn try_reset_executors(&self) -> WorkflowResult<()> {
        let has_resettable = self
            .registrations
            .values()
            .any(|reg| reg.supports_resetting);

        if !has_resettable {
            return Ok(());
        }

        // For now, we'll just mark that reset was attempted
        // In a full implementation, we'd track executor instances and reset them
        self.needs_reset
            .store(false, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Describe the protocol for interacting with this workflow
    pub async fn describe_protocol(&self) -> WorkflowResult<ProtocolDescriptor> {
        let start_registration = self
            .registrations
            .get(&self.start_executor_id)
            .ok_or_else(|| {
                WorkflowError::ExecutorNotFound(self.start_executor_id.clone())
            })?;

        let start_executor = start_registration.create_instance().await?;
        Ok(start_executor.describe_protocol())
    }

    /// Get workflow information for serialization/inspection
    pub fn to_workflow_info(&self) -> WorkflowInfo {
        WorkflowInfo {
            start_executor_id: self.start_executor_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            executor_ids: self.registrations.keys().cloned().collect(),
            edges: self.reflect_edges(),
            output_executor_ids: self.output_executors.iter().cloned().collect(),
            allows_concurrent: self.allow_concurrent(),
        }
    }
}

/// Token representing ownership of a workflow
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipToken {
    pub owner_id: String,
    pub is_subworkflow: bool,
}

impl OwnershipToken {
    pub fn new(owner_id: String) -> Self {
        Self {
            owner_id,
            is_subworkflow: false,
        }
    }

    pub fn subworkflow(owner_id: String) -> Self {
        Self {
            owner_id,
            is_subworkflow: true,
        }
    }
}

/// Information about a request port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPortInfo {
    pub id: String,
    pub description: Option<String>,
    pub input_types: Vec<String>,
}

/// Serializable information about a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub start_executor_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub executor_ids: Vec<String>,
    pub edges: HashMap<String, Vec<EdgeInfo>>,
    pub output_executor_ids: Vec<String>,
    pub allows_concurrent: bool,
}