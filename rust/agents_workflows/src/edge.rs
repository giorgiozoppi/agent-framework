// Copyright (c) Microsoft. All rights reserved.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Unique identifier for an edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub u64);

impl EdgeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// An edge connecting two executors in a workflow
#[derive(Debug, Clone)]
pub struct Edge {
    pub data: EdgeData,
}

impl Edge {
    pub fn direct(source: String, target: String, id: EdgeId) -> Self {
        Self {
            data: EdgeData::Direct(DirectEdgeData {
                source_id: source,
                target_id: target,
                id,
                condition: None,
            }),
        }
    }

    pub fn direct_with_condition<F>(
        source: String,
        target: String,
        id: EdgeId,
        condition: F,
    ) -> Self
    where
        F: Fn(&Value) -> bool + Send + Sync + 'static,
    {
        Self {
            data: EdgeData::Direct(DirectEdgeData {
                source_id: source,
                target_id: target,
                id,
                condition: Some(Arc::new(condition)),
            }),
        }
    }

    pub fn fan_out(source: String, targets: Vec<String>, id: EdgeId) -> Self {
        Self {
            data: EdgeData::FanOut(FanOutEdgeData {
                source_id: source,
                target_ids: targets,
                id,
                partitioner: None,
            }),
        }
    }

    pub fn fan_out_with_partitioner<F>(
        source: String,
        targets: Vec<String>,
        id: EdgeId,
        partitioner: F,
    ) -> Self
    where
        F: Fn(&Value, usize) -> Vec<usize> + Send + Sync + 'static,
    {
        Self {
            data: EdgeData::FanOut(FanOutEdgeData {
                source_id: source,
                target_ids: targets,
                id,
                partitioner: Some(Arc::new(partitioner)),
            }),
        }
    }

    pub fn fan_in(sources: Vec<String>, target: String, id: EdgeId) -> Self {
        Self {
            data: EdgeData::FanIn(FanInEdgeData {
                source_ids: sources,
                target_id: target,
                id,
            }),
        }
    }

    pub fn id(&self) -> EdgeId {
        match &self.data {
            EdgeData::Direct(d) => d.id,
            EdgeData::FanOut(d) => d.id,
            EdgeData::FanIn(d) => d.id,
        }
    }

    pub fn source_ids(&self) -> Vec<&str> {
        match &self.data {
            EdgeData::Direct(d) => vec![&d.source_id],
            EdgeData::FanOut(d) => vec![&d.source_id],
            EdgeData::FanIn(d) => d.source_ids.iter().map(|s| s.as_str()).collect(),
        }
    }

    pub fn target_ids(&self) -> Vec<&str> {
        match &self.data {
            EdgeData::Direct(d) => vec![&d.target_id],
            EdgeData::FanOut(d) => d.target_ids.iter().map(|s| s.as_str()).collect(),
            EdgeData::FanIn(d) => vec![&d.target_id],
        }
    }
}

/// Edge data variants
#[derive(Debug, Clone)]
pub enum EdgeData {
    Direct(DirectEdgeData),
    FanOut(FanOutEdgeData),
    FanIn(FanInEdgeData),
}

/// Direct edge between two executors
#[derive(Clone)]
pub struct DirectEdgeData {
    pub source_id: String,
    pub target_id: String,
    pub id: EdgeId,
    pub condition: Option<Arc<dyn Fn(&Value) -> bool + Send + Sync>>,
}

impl std::fmt::Debug for DirectEdgeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectEdgeData")
            .field("source_id", &self.source_id)
            .field("target_id", &self.target_id)
            .field("id", &self.id)
            .field("condition", &self.condition.is_some())
            .finish()
    }
}

/// Fan-out edge from one executor to multiple targets
#[derive(Clone)]
pub struct FanOutEdgeData {
    pub source_id: String,
    pub target_ids: Vec<String>,
    pub id: EdgeId,
    pub partitioner: Option<Arc<dyn Fn(&Value, usize) -> Vec<usize> + Send + Sync>>,
}

impl std::fmt::Debug for FanOutEdgeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FanOutEdgeData")
            .field("source_id", &self.source_id)
            .field("target_ids", &self.target_ids)
            .field("id", &self.id)
            .field("partitioner", &self.partitioner.is_some())
            .finish()
    }
}

/// Fan-in edge from multiple sources to one target
#[derive(Debug, Clone)]
pub struct FanInEdgeData {
    pub source_ids: Vec<String>,
    pub target_id: String,
    pub id: EdgeId,
}

/// Information about an edge for reflection/inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInfo {
    pub id: u64,
    pub edge_type: EdgeType,
    pub source_ids: Vec<String>,
    pub target_ids: Vec<String>,
    pub has_condition: bool,
    pub has_partitioner: bool,
}

/// Types of edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Direct,
    FanOut,
    FanIn,
}

impl From<&Edge> for EdgeInfo {
    fn from(edge: &Edge) -> Self {
        match &edge.data {
            EdgeData::Direct(data) => EdgeInfo {
                id: data.id.0,
                edge_type: EdgeType::Direct,
                source_ids: vec![data.source_id.clone()],
                target_ids: vec![data.target_id.clone()],
                has_condition: data.condition.is_some(),
                has_partitioner: false,
            },
            EdgeData::FanOut(data) => EdgeInfo {
                id: data.id.0,
                edge_type: EdgeType::FanOut,
                source_ids: vec![data.source_id.clone()],
                target_ids: data.target_ids.clone(),
                has_condition: false,
                has_partitioner: data.partitioner.is_some(),
            },
            EdgeData::FanIn(data) => EdgeInfo {
                id: data.id.0,
                edge_type: EdgeType::FanIn,
                source_ids: data.source_ids.clone(),
                target_ids: vec![data.target_id.clone()],
                has_condition: false,
                has_partitioner: false,
            },
        }
    }
}