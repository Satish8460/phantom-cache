#[cfg(test)]
mod tests {
    use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
    use phantom_cache_demo::dist::router::RendezvousRouter;
    use std::sync::Arc;

    #[test]
    fn test_normal_routing() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);

        let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
        let primary = router.route_key("hotel_123");
        assert!(primary.is_some());
    }

    #[test]
    fn test_node_failure_exclusion() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Failed); // Node 2 is marked offline

        let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);

        // Ensure keys always map to active Node 1
        for i in 0..10 {
            let key = format!("hotel_{}", i);
            let primary = router.route_key(&key).unwrap();
            assert_eq!(primary.node_id, 1);
        }
    }

    #[test]
    fn test_replica_selection() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);
        n3.set_state(NodeState::Active);

        let router = RendezvousRouter::new(vec![n1, n2, n3]);
        let replicas = router.get_replicas("hotel_123", 1);
        assert_eq!(replicas.len(), 1);
    }
}
