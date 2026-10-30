use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CoalesceKey {
    pub path: String,
    pub auth_token_hash: String,
    pub tenant_id: String,
}

pub struct Coalescer {
    in_flight: Mutex<HashMap<CoalesceKey, Vec<oneshot::Sender<Vec<u8>>>>>,
    waiter_limit: usize,
}

impl Coalescer {
    pub fn new(waiter_limit: usize) -> Self {
        Self {
            in_flight: Mutex::new(HashMap::new()),
            waiter_limit,
        }
    }

    /// Registers interest in a key. Returns Some(receiver) if we are a waiter,
    /// or None if we are the first requester (responsible for fetching the value).
    pub fn register(
        &self,
        key: CoalesceKey,
    ) -> Result<Option<oneshot::Receiver<Vec<u8>>>, &'static str> {
        let mut guard = self.in_flight.lock().unwrap();

        if let Some(waiters) = guard.get_mut(&key) {
            if waiters.len() >= self.waiter_limit {
                return Err("Waiter limit reached");
            }
            let (tx, rx) = oneshot::channel();
            waiters.push(tx);
            Ok(Some(rx))
        } else {
            guard.insert(key, Vec::new());
            Ok(None)
        }
    }

    /// Resolves the coalescing loop by publishing the value to all waiters.
    pub fn resolve(&self, key: &CoalesceKey, value: Vec<u8>) {
        let mut guard = self.in_flight.lock().unwrap();
        if let Some(waiters) = guard.remove(key) {
            for tx in waiters {
                let _ = tx.send(value.clone());
            }
        }
    }
}
