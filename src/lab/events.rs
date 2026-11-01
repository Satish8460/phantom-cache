//! Deterministic event engine with total ordering `(time_us, sequence_id)` (M4-01).
//!
//! Provides min-heap event scheduling. If two events have identical virtual timestamps,
//! tie-breaking is strictly determined by monotonic `sequence_id`, eliminating non-determinism.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventId {
    pub time_us: u64,
    pub seq_id: u64,
}

impl Ord for EventId {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering so BinaryHeap acts as a min-heap
        other
            .time_us
            .cmp(&self.time_us)
            .then_with(|| other.seq_id.cmp(&self.seq_id))
    }
}

impl PartialOrd for EventId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SimEvent<T> {
    pub id: EventId,
    pub payload: T,
}

impl<T> Ord for SimEvent<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T> PartialOrd for SimEvent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for SimEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for SimEvent<T> {}

pub struct EventQueue<T> {
    heap: BinaryHeap<SimEvent<T>>,
    next_seq_id: u64,
}

impl<T> Default for EventQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EventQueue<T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_seq_id: 0,
        }
    }

    pub fn schedule(&mut self, time_us: u64, payload: T) -> EventId {
        let seq_id = self.next_seq_id;
        self.next_seq_id = self.next_seq_id.wrapping_add(1);

        let id = EventId { time_us, seq_id };
        self.heap.push(SimEvent { id, payload });
        id
    }

    pub fn pop(&mut self) -> Option<SimEvent<T>> {
        self.heap.pop()
    }

    pub fn peek_time(&self) -> Option<u64> {
        self.heap.peek().map(|e| e.id.time_us)
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

