use crate::dist::node::{NodeState, VirtualNode};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub struct RendezvousRouter {
    nodes: Vec<Arc<VirtualNode>>,
}

impl RendezvousRouter {
    pub fn new(nodes: Vec<Arc<VirtualNode>>) -> Self {
        Self { nodes }
    }

    fn calculate_hash(key: &str, node_id: u8) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        node_id.hash(&mut hasher);
        hasher.finish()
    }

    pub fn route_key(&self, key: &str) -> Option<Arc<VirtualNode>> {
        let mut highest_weight = 0.0;
        let mut selected_node: Option<Arc<VirtualNode>> = None;

        for node in &self.nodes {
            let state = node.get_state();
            if state == NodeState::Failed || state == NodeState::Offline {
                continue; // Exclude offline/failed nodes from partition mapping
            }

            let hash_val = Self::calculate_hash(key, node.node_id);
            // Dynamic capacity weights (assume default scale 1.0 for simplicity)
            let capacity_weight = 1.0;
            let weight = (hash_val as f64) * capacity_weight;

            if weight > highest_weight {
                highest_weight = weight;
                selected_node = Some(node.clone());
            }
        }

        selected_node
    }

    pub fn get_replicas(&self, key: &str, count: usize) -> Vec<Arc<VirtualNode>> {
        let mut weighted_nodes: Vec<(f64, Arc<VirtualNode>)> = self
            .nodes
            .iter()
            .filter(|node| {
                let s = node.get_state();
                s != NodeState::Failed && s != NodeState::Offline
            })
            .map(|node| {
                let hash_val = Self::calculate_hash(key, node.node_id);
                (hash_val as f64, node.clone())
            })
            .collect();

        // Sort descending by weight value
        weighted_nodes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        weighted_nodes
            .into_iter()
            .skip(1) // Skip primary
            .take(count)
            .map(|(_, node)| node)
            .collect()
    }
}
