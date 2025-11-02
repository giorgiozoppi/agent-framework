// Copyright (c) Microsoft. All rights reserved.

use crate::{Edge, EdgeData, WorkflowResult};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Routes messages between executors based on workflow edges
#[derive(Debug)]
pub struct MessageRouter {
    /// Edges organized by source executor ID
    edges: HashMap<String, Vec<Edge>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// Add an edge to the router
    pub fn add_edge(&mut self, edge: Edge) {
        for source_id in edge.source_ids() {
            self.edges
                .entry(source_id.to_string())
                .or_default()
                .push(edge.clone());
        }
    }

    /// Route a message from a source executor
    pub fn route_message(
        &self,
        source_id: &str,
        message: &Value,
        target_id: Option<&str>,
    ) -> WorkflowResult<Vec<RouteTarget>> {
        let mut targets = Vec::new();

        if let Some(edges) = self.edges.get(source_id) {
            for edge in edges {
                match &edge.data {
                    EdgeData::Direct(data) => {
                        // Check if target matches (if specified)
                        if let Some(target) = target_id {
                            if data.target_id != target {
                                continue;
                            }
                        }

                        // Check condition if present
                        if let Some(condition) = &data.condition {
                            if !condition(message) {
                                continue;
                            }
                        }

                        targets.push(RouteTarget {
                            executor_id: data.target_id.clone(),
                            message: message.clone(),
                        });
                    }
                    EdgeData::FanOut(data) => {
                        // Check if any target matches (if specified)
                        if let Some(target) = target_id {
                            if !data.target_ids.contains(&target.to_string()) {
                                continue;
                            }
                        }

                        let target_indices = if let Some(partitioner) = &data.partitioner {
                            partitioner(message, data.target_ids.len())
                        } else {
                            // Send to all targets by default
                            (0..data.target_ids.len()).collect()
                        };

                        for index in target_indices {
                            if index < data.target_ids.len() {
                                let target_executor = &data.target_ids[index];

                                // Filter by target if specified
                                if let Some(target) = target_id {
                                    if target_executor != target {
                                        continue;
                                    }
                                }

                                targets.push(RouteTarget {
                                    executor_id: target_executor.clone(),
                                    message: message.clone(),
                                });
                            }
                        }
                    }
                    EdgeData::FanIn(data) => {
                        // For fan-in, only route if this is the correct source
                        if !data.source_ids.contains(&source_id.to_string()) {
                            continue;
                        }

                        // Check if target matches (if specified)
                        if let Some(target) = target_id {
                            if data.target_id != target {
                                continue;
                            }
                        }

                        targets.push(RouteTarget {
                            executor_id: data.target_id.clone(),
                            message: message.clone(),
                        });
                    }
                }
            }
        }

        Ok(targets)
    }

    /// Get all possible target executor IDs from a source
    pub fn get_targets(&self, source_id: &str) -> HashSet<String> {
        let mut targets = HashSet::new();

        if let Some(edges) = self.edges.get(source_id) {
            for edge in edges {
                for target_id in edge.target_ids() {
                    targets.insert(target_id.to_string());
                }
            }
        }

        targets
    }

    /// Get all source executor IDs that can reach a target
    pub fn get_sources(&self, target_id: &str) -> HashSet<String> {
        let mut sources = HashSet::new();

        for (source_id, edges) in &self.edges {
            for edge in edges {
                if edge.target_ids().contains(&target_id) {
                    sources.insert(source_id.clone());
                }
            }
        }

        sources
    }

    /// Check if there's a path from source to target
    pub fn has_path(&self, source_id: &str, target_id: &str) -> bool {
        // Simple direct check - could be enhanced with transitive path finding
        self.get_targets(source_id).contains(target_id)
    }

    /// Get all edges from a source
    pub fn get_edges_from(&self, source_id: &str) -> Vec<&Edge> {
        self.edges
            .get(source_id)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get statistics about the routing table
    pub fn get_stats(&self) -> RouterStats {
        let mut total_edges = 0;
        let mut executors_with_outgoing = 0;
        let mut all_targets = HashSet::new();

        for (_, edges) in &self.edges {
            if !edges.is_empty() {
                executors_with_outgoing += 1;
            }
            for edge in edges {
                total_edges += 1;
                for target in edge.target_ids() {
                    all_targets.insert(target.to_string());
                }
            }
        }

        RouterStats {
            total_edges,
            executors_with_outgoing,
            total_target_executors: all_targets.len(),
        }
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Target for message routing
#[derive(Debug, Clone)]
pub struct RouteTarget {
    pub executor_id: String,
    pub message: Value,
}

/// Statistics about the message router
#[derive(Debug)]
pub struct RouterStats {
    pub total_edges: usize,
    pub executors_with_outgoing: usize,
    pub total_target_executors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeId, Edge};
    use serde_json::json;

    #[test]
    fn test_direct_routing() {
        let mut router = MessageRouter::new();
        let edge = Edge::direct("A".to_string(), "B".to_string(), EdgeId::new(1));
        router.add_edge(edge);

        let message = json!({"test": "data"});
        let targets = router.route_message("A", &message, None).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].executor_id, "B");
    }

    #[test]
    fn test_fan_out_routing() {
        let mut router = MessageRouter::new();
        let edge = Edge::fan_out(
            "A".to_string(),
            vec!["B".to_string(), "C".to_string()],
            EdgeId::new(1),
        );
        router.add_edge(edge);

        let message = json!({"test": "data"});
        let targets = router.route_message("A", &message, None).unwrap();

        assert_eq!(targets.len(), 2);
        let target_ids: HashSet<_> = targets.iter().map(|t| &t.executor_id).collect();
        assert!(target_ids.contains(&"B".to_string()));
        assert!(target_ids.contains(&"C".to_string()));
    }

    #[test]
    fn test_conditional_routing() {
        let mut router = MessageRouter::new();
        let edge = Edge::direct_with_condition(
            "A".to_string(),
            "B".to_string(),
            EdgeId::new(1),
            |msg| msg.get("condition").and_then(|v| v.as_bool()).unwrap_or(false),
        );
        router.add_edge(edge);

        // Message that should pass condition
        let message_pass = json!({"condition": true});
        let targets = router.route_message("A", &message_pass, None).unwrap();
        assert_eq!(targets.len(), 1);

        // Message that should fail condition
        let message_fail = json!({"condition": false});
        let targets = router.route_message("A", &message_fail, None).unwrap();
        assert_eq!(targets.len(), 0);
    }
}