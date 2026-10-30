use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub time_ms: u64,
    pub task_id: u64,
    pub description: String,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.time_ms == other.time_ms
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap order to execute earliest events first
        other.time_ms.cmp(&self.time_ms)
    }
}

pub struct EventScheduler {
    heap: Mutex<BinaryHeap<ScheduledTask>>,
    next_task_id: Mutex<u64>,
}

impl Default for EventScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventScheduler {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            next_task_id: Mutex::new(0),
        }
    }

    pub fn schedule(&self, time_ms: u64, description: &str) -> u64 {
        let mut id_guard = self.next_task_id.lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;

        let mut heap_guard = self.heap.lock().unwrap();
        heap_guard.push(ScheduledTask {
            time_ms,
            task_id: id,
            description: description.to_string(),
        });
        id
    }

    pub fn pop_next_before(&self, max_time_ms: u64) -> Option<ScheduledTask> {
        let mut heap_guard = self.heap.lock().unwrap();
        if let Some(next) = heap_guard.peek() {
            if next.time_ms <= max_time_ms {
                return heap_guard.pop();
            }
        }
        None
    }

    pub fn clear(&self) {
        let mut heap_guard = self.heap.lock().unwrap();
        heap_guard.clear();
        let mut id_guard = self.next_task_id.lock().unwrap();
        *id_guard = 0;
    }
}
