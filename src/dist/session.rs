use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

type SessionMap = HashMap<u64, (u8, watch::Sender<bool>)>;

/// Tracks active inbound cluster sessions on a node.
/// Enforces Layer 5 of the Phase 3 trust model: when a peer node is revoked or quarantined
/// in the membership registry, all open sockets associated with that peer are cancelled immediately.
#[derive(Clone, Default)]
pub struct SessionTracker {
    sessions: Arc<Mutex<SessionMap>>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a newly authenticated inbound connection session.
    /// Returns the session ID and a watch receiver that will be notified if the session is revoked.
    pub fn register(&self, peer_node_id: u8) -> (u64, watch::Receiver<bool>) {
        let session_id = rand::random::<u64>();
        let (tx, rx) = watch::channel(false);
        let mut guard = self.sessions.lock().unwrap();
        guard.insert(session_id, (peer_node_id, tx));
        (session_id, rx)
    }

    /// Unregisters a cleanly terminated connection session.
    pub fn unregister(&self, session_id: u64) {
        let mut guard = self.sessions.lock().unwrap();
        guard.remove(&session_id);
    }

    /// Actively evicts all open sessions for a revoked peer node ID.
    /// Sends a cancellation signal on each session's watch channel, prompting the listener
    /// task to immediately terminate the connection loop and drop the socket.
    /// Returns the number of evicted sessions.
    pub fn evict_peer(&self, peer_node_id: u8) -> usize {
        let mut guard = self.sessions.lock().unwrap();
        let mut evicted_ids = Vec::new();

        for (&session_id, (node_id, tx)) in guard.iter() {
            if *node_id == peer_node_id {
                let _ = tx.send(true);
                evicted_ids.push(session_id);
            }
        }

        for id in &evicted_ids {
            guard.remove(id);
        }

        evicted_ids.len()
    }

    /// Actively evicts all open sessions across all peers (e.g. on service shutdown).
    pub fn evict_all(&self) -> usize {
        let mut guard = self.sessions.lock().unwrap();
        let count = guard.len();
        for (_, tx) in guard.values() {
            let _ = tx.send(true);
        }
        guard.clear();
        count
    }

    /// Returns the count of active sessions for a specific peer node ID.
    pub fn active_sessions_for_peer(&self, peer_node_id: u8) -> usize {
        let guard = self.sessions.lock().unwrap();
        guard
            .values()
            .filter(|(node_id, _)| *node_id == peer_node_id)
            .count()
    }

    /// Returns the total count of active sessions across all peers.
    pub fn total_active_sessions(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_tracker_registration_and_eviction() {
        let tracker = SessionTracker::new();

        let (s1, mut rx1) = tracker.register(2);
        let (_s2, mut rx2) = tracker.register(2);
        let (_s3, mut rx3) = tracker.register(3);

        assert_eq!(tracker.active_sessions_for_peer(2), 2);
        assert_eq!(tracker.active_sessions_for_peer(3), 1);
        assert_eq!(tracker.total_active_sessions(), 3);

        // Evict Node 2
        let count = tracker.evict_peer(2);
        assert_eq!(count, 2);
        assert_eq!(tracker.active_sessions_for_peer(2), 0);
        assert_eq!(tracker.active_sessions_for_peer(3), 1);

        // Check cancellation signals
        assert!(*rx1.borrow_and_update());
        assert!(*rx2.borrow_and_update());
        assert!(!*rx3.borrow_and_update());

        // Unregister remaining
        tracker.unregister(s1);
    }
}
