// Copyright (c) Microsoft. All rights reserved.

use crate::{WorkflowError, WorkflowResult};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

/// Manages workflow state across execution steps
#[derive(Debug, Clone)]
pub struct StateManager {
    /// State organized by scope -> key -> value
    state: Arc<DashMap<String, DashMap<String, Value>>>,
    /// Queued state updates for next super step
    queued_updates: Arc<DashMap<StateKey, StateUpdate>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            queued_updates: Arc::new(DashMap::new()),
        }
    }

    /// Read a state value from the given scope and key
    pub async fn read_state<T>(&self, scope: &str, key: &str) -> WorkflowResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let scope_map = self.state.get(scope);
        if let Some(scope_map) = scope_map {
            if let Some(value) = scope_map.get(key) {
                let result: T = serde_json::from_value(value.clone())
                    .map_err(|e| WorkflowError::state_error(format!("Failed to deserialize state value for key '{}' in scope '{}': {}", key, scope, e)))?;
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Read state value or initialize with factory if not present
    pub async fn read_or_init_state<T, F>(&self, scope: &str, key: &str, factory: F) -> WorkflowResult<T>
    where
        T: for<'de> Deserialize<'de> + Serialize,
        F: FnOnce() -> T,
    {
        if let Some(value) = self.read_state(scope, key).await? {
            Ok(value)
        } else {
            let value = factory();
            self.queue_state_update(scope, key, &value).await?;
            Ok(value)
        }
    }

    /// Get all keys in a scope
    pub async fn read_state_keys(&self, scope: &str) -> WorkflowResult<HashSet<String>> {
        if let Some(scope_map) = self.state.get(scope) {
            Ok(scope_map.iter().map(|entry| entry.key().clone()).collect())
        } else {
            Ok(HashSet::new())
        }
    }

    /// Queue a state update for the next super step
    pub async fn queue_state_update<T>(&self, scope: &str, key: &str, value: &T) -> WorkflowResult<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| WorkflowError::state_error(format!("Failed to serialize state value: {}", e)))?;

        let state_key = StateKey {
            scope: scope.to_string(),
            key: key.to_string(),
        };

        self.queued_updates.insert(state_key, StateUpdate::Set(json_value));
        Ok(())
    }

    /// Queue clearing a scope
    pub async fn queue_clear_scope(&self, scope: &str) -> WorkflowResult<()> {
        let state_key = StateKey {
            scope: scope.to_string(),
            key: String::new(), // Empty key indicates scope-level operation
        };

        self.queued_updates.insert(state_key, StateUpdate::ClearScope);
        Ok(())
    }

    /// Apply all queued state updates
    pub async fn apply_queued_updates(&self) -> WorkflowResult<()> {
        for entry in self.queued_updates.iter() {
            let state_key = entry.key();
            let update = entry.value();

            match update {
                StateUpdate::Set(value) => {
                    let scope_map = self.state.entry(state_key.scope.clone())
                        .or_insert_with(DashMap::new);
                    scope_map.insert(state_key.key.clone(), value.clone());
                }
                StateUpdate::Delete => {
                    if let Some(scope_map) = self.state.get(&state_key.scope) {
                        scope_map.remove(&state_key.key);
                    }
                }
                StateUpdate::ClearScope => {
                    self.state.remove(&state_key.scope);
                }
            }
        }

        self.queued_updates.clear();
        Ok(())
    }

    /// Get a snapshot of all current state
    pub fn get_snapshot(&self) -> StateSnapshot {
        let mut snapshot = StateSnapshot::new();

        for scope_entry in self.state.iter() {
            let scope = scope_entry.key().clone();
            let scope_map = scope_entry.value();

            for key_entry in scope_map.iter() {
                let key = key_entry.key().clone();
                let value = key_entry.value().clone();
                snapshot.insert(scope.clone(), key, value);
            }
        }

        snapshot
    }

    /// Restore state from a snapshot
    pub fn restore_from_snapshot(&self, snapshot: &StateSnapshot) {
        self.state.clear();

        for (scope, scope_data) in &snapshot.data {
            let scope_map = self.state.entry(scope.clone())
                .or_insert_with(DashMap::new);

            for (key, value) in scope_data {
                scope_map.insert(key.clone(), value.clone());
            }
        }
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Key for state storage
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StateKey {
    scope: String,
    key: String,
}

/// State update operation
#[derive(Debug, Clone)]
enum StateUpdate {
    Set(Value),
    Delete,
    ClearScope,
}

/// Snapshot of state for checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    data: std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
}

impl StateSnapshot {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    fn insert(&mut self, scope: String, key: String, value: Value) {
        self.data.entry(scope).or_default().insert(key, value);
    }
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self::new()
    }
}