use crate::dist::frame::{
    AntiEntropyStatus, InvalidateRecord, MAX_FRAME_PAYLOAD_BYTES, MAX_KEY_LENGTH_BYTES,
};
use crate::intel::diagnostics::CacheCounters;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::Ordering;

/// Default maximum number of invalidation records retained in the ring buffer.
pub const DEFAULT_MAX_RING_ENTRIES: usize = 4_096;

/// Default hard memory ceiling for the ring buffer (4 MB = 4,194,304 bytes).
pub const DEFAULT_MAX_RING_BYTES: usize = 4_194_304;

/// Capacity of the SEPARATE accounting list for inbound records (never replayed).
pub const INBOUND_ACCOUNTING_ENTRIES: usize = 256;

/// Where a ring record came from / who may be served it by a replay.
///
/// The ring is a LOCAL structure (never sent as such); the wire format is unchanged. Each
/// destination has its own sequence space, so a record's `stream_seq` is only meaningful together
/// with its destination: replaying records of another destination's stream to a requester would
/// poison the requester's tracker and apply foreign invalidations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingDest {
    /// Sent BY this node TO the given peer (its per-destination stream).
    To(u8),
    /// Received from a peer: kept for diagnostics/accounting, never replayed.
    Inbound,
    /// Legacy/unscoped record (`push`): replayable to any requester.
    Any,
}

struct RingEntry {
    rec: InvalidateRecord,
    dest: RingDest,
}

/// In-memory dual-bounded circular ring buffer storing recent invalidation records.
/// Enforces both entry count and byte ceiling limits to guarantee bounded memory consumption
/// during high invalidation rates or prolonged network partitions.
///
/// Replayable records (outbound, per destination) live in `records`. Inbound records are kept in
/// a SEPARATE small bounded list (`INBOUND_ACCOUNTING_ENTRIES`) so that a flood of received
/// invalidations can never evict replayable history.
pub struct InvalidationRingBuffer {
    records: VecDeque<RingEntry>,
    inbound: VecDeque<RingEntry>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    oldest_seq: u64,
    newest_seq: u64,
    /// Highest sequence ever evicted, over all replayable records (legacy `get_replay_batch`).
    evicted_max_any: u64,
    /// Highest sequence ever evicted per outbound destination stream.
    evicted_max_to: HashMap<u8, u64>,
}

impl Default for InvalidationRingBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_RING_ENTRIES, DEFAULT_MAX_RING_BYTES)
    }
}

impl InvalidationRingBuffer {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_entries.min(4096)),
            inbound: VecDeque::new(),
            max_entries,
            max_bytes,
            current_bytes: 0,
            oldest_seq: 0,
            newest_seq: 0,
            evicted_max_any: 0,
            evicted_max_to: HashMap::new(),
        }
    }

    /// Legacy unscoped append (replayable to every requester).
    pub fn push(&mut self, record: InvalidateRecord) {
        self.push_replayable(record, RingDest::Any);
    }

    /// Records an invalidation THIS node sent to `dest` (its per-destination stream).
    pub fn push_outbound(&mut self, record: InvalidateRecord, dest: u8) {
        self.push_replayable(record, RingDest::To(dest));
    }

    /// Records an invalidation received from a peer (accounting only; never replayed). Kept in a
    /// separate bounded list; a sender retry of the same (sequence, scope, key) is not recorded twice.
    pub fn push_inbound(&mut self, record: InvalidateRecord) {
        if self
            .inbound
            .iter()
            .rev()
            .any(|e| e.rec.stream_seq == record.stream_seq && e.rec.scope == record.scope && e.rec.key == record.key)
        {
            return;
        }
        let size = 11 + record.key.len();
        while self.inbound.len() >= INBOUND_ACCOUNTING_ENTRIES {
            if let Some(p) = self.inbound.pop_front() {
                self.current_bytes = self.current_bytes.saturating_sub(11 + p.rec.key.len());
            }
        }
        if self.records.is_empty() && self.inbound.is_empty() {
            self.oldest_seq = record.stream_seq;
        } else {
            self.oldest_seq = self.oldest_seq.min(record.stream_seq);
        }
        self.newest_seq = self.newest_seq.max(record.stream_seq);
        self.current_bytes = self.current_bytes.saturating_add(size);
        self.inbound.push_back(RingEntry { rec: record, dest: RingDest::Inbound });
    }

    fn recompute_oldest(&mut self, fallback: u64) {
        self.oldest_seq = self
            .records
            .iter()
            .chain(self.inbound.iter())
            .map(|e| e.rec.stream_seq)
            .min()
            .unwrap_or(fallback);
    }

    fn push_replayable(&mut self, record: InvalidateRecord, dest: RingDest) {
        let entry_size = 11 + record.key.len();

        while !self.records.is_empty()
            && (self.records.len() >= self.max_entries
                || self.current_bytes.saturating_add(entry_size) > self.max_bytes)
        {
            if let Some(popped) = self.records.pop_front() {
                let popped_size = 11 + popped.rec.key.len();
                self.current_bytes = self.current_bytes.saturating_sub(popped_size);
                self.evicted_max_any = self.evicted_max_any.max(popped.rec.stream_seq);
                if let RingDest::To(d) = popped.dest {
                    let e = self.evicted_max_to.entry(d).or_insert(0);
                    *e = (*e).max(popped.rec.stream_seq);
                }
                // The minimum only changes when the evicted record could have been the minimum.
                if popped.rec.stream_seq <= self.oldest_seq {
                    self.recompute_oldest(record.stream_seq);
                }
            }
        }

        if self.records.is_empty() && self.inbound.is_empty() {
            self.oldest_seq = record.stream_seq;
        } else {
            self.oldest_seq = self.oldest_seq.min(record.stream_seq);
        }
        // Monotone: a re-delivery (repair) carries its ORIGINAL, possibly lower, sequence and must
        // not move the newest marker backwards.
        self.newest_seq = self.newest_seq.max(record.stream_seq);
        self.current_bytes = self.current_bytes.saturating_add(entry_size);
        self.records.push_back(RingEntry { rec: record, dest });
    }

    /// Computes the estimated total resident memory footprint (in bytes) of this ring buffer.
    pub fn estimated_resident_memory_bytes(&self) -> usize {
        let payload_bytes = self.current_bytes;
        let collection_bytes = (self.records.capacity() + self.inbound.capacity())
            * std::mem::size_of::<RingEntry>();
        let string_overhead_bytes = (self.records.len() + self.inbound.len()) * 16;
        payload_bytes
            .saturating_add(collection_bytes)
            .saturating_add(string_overhead_bytes)
    }

    /// Computes a deterministic 32-byte SHA-256 digest over the current active sequence window.
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
        ctx.update(&self.oldest_seq.to_be_bytes());
        ctx.update(&self.newest_seq.to_be_bytes());
        ctx.update(&((self.records.len() + self.inbound.len()) as u32).to_be_bytes());

        for e in self.records.iter().chain(self.inbound.iter()) {
            let rec = &e.rec;
            ctx.update(&rec.stream_seq.to_be_bytes());
            ctx.update(&[rec.scope.to_u8()]);
            ctx.update(&(rec.key.len() as u16).to_be_bytes());
            ctx.update(rec.key.as_bytes());
        }

        let digest = ctx.finish();
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_ref());
        out
    }

    /// Replayable records with `stream_seq >= needed_seq` accepted by `keep`, ordered by sequence
    /// (insertion order is not sequence order), truncated to the frame payload budget. The result
    /// is therefore a sequence-ordered PREFIX of what is owed: its last record's sequence is the
    /// contiguous point actually delivered.
    fn collect_batch(
        &self,
        needed_seq: u64,
        mut keep: impl FnMut(&RingEntry) -> bool,
    ) -> Vec<InvalidateRecord> {
        let mut matching: Vec<&RingEntry> = self
            .records
            .iter()
            .filter(|e| e.rec.stream_seq >= needed_seq && keep(e))
            .collect();
        matching.sort_by_key(|e| e.rec.stream_seq);
        let mut batch = Vec::new();
        let mut batch_bytes = 20usize; // Base header length
        for e in matching {
            let rec_size = 11 + e.rec.key.len();
            if batch_bytes.saturating_add(rec_size) > MAX_FRAME_PAYLOAD_BYTES as usize {
                break;
            }
            batch.push(e.rec.clone());
            batch_bytes = batch_bytes.saturating_add(rec_size);
        }
        batch
    }

    /// True if a batch of `records` is so close to the frame payload cap that another maximum-size
    /// record may have been cut off: the receiver must then treat the batch as possibly truncated.
    pub fn batch_may_be_truncated(records: &[InvalidateRecord]) -> bool {
        let used: usize = 20 + records.iter().map(|r| 11 + r.key.len()).sum::<usize>();
        used + (11 + MAX_KEY_LENGTH_BYTES) > MAX_FRAME_PAYLOAD_BYTES as usize
    }

    /// Reconstructs a batch of invalidation records for a peer whose highest acknowledged
    /// sequence is `highest_acked_seq` (legacy, unscoped: every replayable record regardless of
    /// destination).
    ///
    /// - If `highest_acked_seq >= newest_seq`, peer is in sync -> returns `Ok(empty)`.
    /// - If a record the peer would need was evicted (`highest_acked_seq` below the highest
    ///   evicted sequence) -> returns `Err(HistoryOverwritten)`.
    /// - Otherwise, returns the missing records with `stream_seq > highest_acked_seq`.
    pub fn get_replay_batch(
        &self,
        highest_acked_seq: u64,
    ) -> Result<Vec<InvalidateRecord>, AntiEntropyStatus> {
        if self.records.is_empty() && self.inbound.is_empty() {
            if highest_acked_seq == 0 {
                return Ok(Vec::new());
            } else {
                return Err(AntiEntropyStatus::HistoryOverwritten);
            }
        }
        if highest_acked_seq >= self.newest_seq {
            return Ok(Vec::new());
        }
        if highest_acked_seq < self.evicted_max_any {
            return Err(AntiEntropyStatus::HistoryOverwritten);
        }
        Ok(self.collect_batch(highest_acked_seq.saturating_add(1), |_| true))
    }

    /// Replay for ONE requester: only records of the stream this node sent to `requester`
    /// (plus unscoped legacy records), in that stream's own sequence space.
    ///
    /// `HistoryOverwritten` when
    /// - a record of THAT stream above `highest_acked_seq` was evicted, or
    /// - the requester claims to have received MORE than this node ever sent it
    ///   (`highest_acked_seq` above the stream's newest/evicted sequence): this node's stream
    ///   restarted and its history is gone, so what the requester missed cannot be replayed.
    pub fn get_replay_batch_for(
        &self,
        requester: u8,
        highest_acked_seq: u64,
    ) -> Result<Vec<InvalidateRecord>, AntiEntropyStatus> {
        let evicted = self.evicted_max_to.get(&requester).copied().unwrap_or(0);
        if highest_acked_seq < evicted {
            return Err(AntiEntropyStatus::HistoryOverwritten);
        }
        if highest_acked_seq > self.newest_seq_for(requester).max(evicted) {
            return Err(AntiEntropyStatus::HistoryOverwritten);
        }
        if self.evicted_max_any > 0
            && highest_acked_seq < self.evicted_max_any
            && self.records.iter().any(|e| e.dest == RingDest::Any)
        {
            // Unscoped legacy records share one sequence space: be conservative.
            return Err(AntiEntropyStatus::HistoryOverwritten);
        }
        let needed = highest_acked_seq.saturating_add(1);
        Ok(self.collect_batch(needed, |e| match e.dest {
            RingDest::To(d) => d == requester,
            RingDest::Any => true,
            RingDest::Inbound => false,
        }))
    }

    /// Highest sequence of the stream this node sent to `requester` (plus unscoped records).
    pub fn newest_seq_for(&self, requester: u8) -> u64 {
        self.records
            .iter()
            .filter(|e| matches!(e.dest, RingDest::To(d) if d == requester) || e.dest == RingDest::Any)
            .map(|e| e.rec.stream_seq)
            .max()
            .unwrap_or(0)
    }

    /// Smallest sequence currently retained (not necessarily the front record's).
    pub fn oldest_seq(&self) -> u64 {
        self.oldest_seq
    }

    pub fn newest_seq(&self) -> u64 {
        self.newest_seq
    }

    pub fn len(&self) -> usize {
        self.records.len() + self.inbound.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.inbound.is_empty()
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.inbound.clear();
        self.current_bytes = 0;
        self.oldest_seq = 0;
        self.newest_seq = 0;
        self.evicted_max_any = 0;
        self.evicted_max_to.clear();
    }
}

/// Maximum number of recorded gap ranges per tracker before a peer is collapsed to "resync needed".
pub const MAX_TRACKED_GAPS: usize = 256;
/// Maximum number of buffered out-of-order sequences per peer before collapsing to "resync needed".
pub const MAX_OUT_OF_ORDER_PER_PEER: usize = 1024;

/// Tracks per-peer expected stream sequence numbers and detects stream sequence gaps.
/// Enforces contiguous acknowledgment semantics: `get_highest_acked_seq` returns the
/// highest contiguous sequence received, preventing gaps from being falsely acknowledged (C-05 / KG-3).
///
/// Memory is bounded: more than `MAX_TRACKED_GAPS` gap ranges, or more than
/// `MAX_OUT_OF_ORDER_PER_PEER` buffered sequences for a peer, collapse that peer's bookkeeping to
/// a single "resync needed" flag (one full pull reconciliation, after which `clear_gaps_for`
/// resets it). A gap that can never be filled (e.g. the sender restarted and lost its history)
/// is likewise cleared by the next successful pull with that peer.
pub struct AntiEntropyTracker {
    expected_seq: HashMap<u8, u64>,
    out_of_order: HashMap<u8, BTreeSet<u64>>,
    detected_gaps: VecDeque<(u8, u64, u64)>, // (peer_node_id, gap_start, gap_end)
    resync_needed: BTreeSet<u8>,
}

impl Default for AntiEntropyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AntiEntropyTracker {
    pub fn new() -> Self {
        Self {
            expected_seq: HashMap::new(),
            out_of_order: HashMap::new(),
            detected_gaps: VecDeque::new(),
            resync_needed: BTreeSet::new(),
        }
    }

    fn collapse_to_resync(&mut self, peer_node_id: u8, stream_seq: u64) {
        self.detected_gaps.retain(|(p, _, _)| *p != peer_node_id);
        self.out_of_order.remove(&peer_node_id);
        self.expected_seq.insert(peer_node_id, stream_seq.saturating_add(1));
        self.resync_needed.insert(peer_node_id);
    }

    /// Records an inbound stream sequence from a peer node.
    /// If `stream_seq > expected`, an anti-entropy gap is detected and recorded, but
    /// `expected` does not advance past the gap. Out-of-order received sequences are buffered
    /// so that when missing sequences arrive, `expected` advances contiguously.
    /// Returns `Some((gap_start, gap_end))` if a gap was detected.
    pub fn record_inbound_seq(
        &mut self,
        peer_node_id: u8,
        stream_seq: u64,
        counters: Option<&CacheCounters>,
    ) -> Option<(u64, u64)> {
        let expected = self.expected_seq.entry(peer_node_id).or_insert(stream_seq);

        if stream_seq > *expected {
            let gap_start = *expected;
            let gap_end = stream_seq.saturating_sub(1);
            if let Some(c) = counters {
                c.anti_entropy_gaps_detected.fetch_add(1, Ordering::Relaxed);
            }
            let ooo_len = {
                let set = self.out_of_order.entry(peer_node_id).or_default();
                set.insert(stream_seq);
                set.len()
            };
            self.detected_gaps.push_back((peer_node_id, gap_start, gap_end));
            if self.detected_gaps.len() > MAX_TRACKED_GAPS || ooo_len > MAX_OUT_OF_ORDER_PER_PEER {
                self.collapse_to_resync(peer_node_id, stream_seq);
            }
            Some((gap_start, gap_end))
        } else if stream_seq == *expected {
            *expected = stream_seq.saturating_add(1);
            if let Some(set) = self.out_of_order.get_mut(&peer_node_id) {
                while set.remove(expected) {
                    *expected = expected.saturating_add(1);
                }
            }
            let now_expected = *expected;
            self.prune_gaps(peer_node_id, now_expected);
            None
        } else {
            // Duplicate or obsolete sequence
            None
        }
    }

    pub fn get_highest_acked_seq(&self, peer_node_id: u8) -> u64 {
        self.expected_seq
            .get(&peer_node_id)
            .map(|s| s.saturating_sub(1))
            .unwrap_or(0)
    }

    pub fn set_highest_acked_seq(&mut self, peer_node_id: u8, seq: u64) {
        let next_expected = seq.saturating_add(1);
        let entry = self.expected_seq.entry(peer_node_id).or_insert(next_expected);
        if next_expected > *entry {
            *entry = next_expected;
            if let Some(set) = self.out_of_order.get_mut(&peer_node_id) {
                set.retain(|&s| s >= next_expected);
                while set.remove(entry) {
                    *entry = entry.saturating_add(1);
                }
            }
            let now_expected = *entry;
            self.prune_gaps(peer_node_id, now_expected);
        }
    }

    /// Removes recorded gaps of `peer_node_id` that lie entirely below the contiguous frontier
    /// `expected` (the late sequences arrived, or a replay advanced past them), so a healed
    /// stream no longer keeps the reconciliation worker running forever.
    fn prune_gaps(&mut self, peer_node_id: u8, expected: u64) {
        self.detected_gaps
            .retain(|(p, _start, end)| !(*p == peer_node_id && *end < expected));
    }

    /// Forgets all gap bookkeeping for a peer (after a successful pull reconciliation with it,
    /// or when the peer left the membership). Sequences the peer can no longer supply stop
    /// triggering reconciliation.
    pub fn clear_gaps_for(&mut self, peer_node_id: u8) {
        self.detected_gaps.retain(|(p, _, _)| *p != peer_node_id);
        self.out_of_order.remove(&peer_node_id);
        self.resync_needed.remove(&peer_node_id);
    }

    pub fn pop_gap(&mut self) -> Option<(u8, u64, u64)> {
        self.detected_gaps.pop_front()
    }

    pub fn has_gaps(&self) -> bool {
        !self.detected_gaps.is_empty() || !self.resync_needed.is_empty()
    }

    /// True if gaps (or a pending resync) exist for this specific peer.
    pub fn has_gaps_for(&self, peer_node_id: u8) -> bool {
        self.resync_needed.contains(&peer_node_id)
            || self.detected_gaps.iter().any(|(p, _, _)| *p == peer_node_id)
    }

    /// Number of recorded gap ranges (bounded by `MAX_TRACKED_GAPS`).
    pub fn gap_count(&self) -> usize {
        self.detected_gaps.len()
    }
}
