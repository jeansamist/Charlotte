//! In-memory list of PipeWire nodes seen by the registry listener.

use super::discovery::NodeKind;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub media_class: String,
    pub kind: NodeKind,
}

#[derive(Debug, Default)]
pub struct DeviceRegistry {
    nodes: BTreeMap<u32, NodeInfo>,
}

impl DeviceRegistry {
    pub fn insert(&mut self, node: NodeInfo) {
        self.nodes.insert(node.id, node);
    }

    pub fn remove(&mut self, id: u32) -> Option<NodeInfo> {
        self.nodes.remove(&id)
    }

    pub fn get(&self, id: u32) -> Option<&NodeInfo> {
        self.nodes.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeInfo> {
        self.nodes.values()
    }
}
