// ui/src/lib/trafficAccounting.js
// Authoritative accounting logic for TrafficGenerator workload simulator.
// Shared between TrafficGenerator.jsx and trafficGenerator.test.mjs (PC-AUTO-030).

import { parseCacheResponse } from './cacheStatus.js';

/**
 * Creates a clean traffic generator stats state object.
 */
export function createTrafficStats() {
  return {
    dispatched: 0,
    completed: 0,
    succeeded: 0,
    failed: 0,
    cancelled: 0,
    inFlight: 0,
    dropped: 0,
    hits: 0,
    misses: 0,
    coalesced: 0,
    others: 0,
  };
}

/**
 * Updates stats when a request is dispatched into flight.
 */
export function recordDispatch(stats, currentInFlight) {
  return {
    ...stats,
    dispatched: stats.dispatched + 1,
    inFlight: currentInFlight,
  };
}

/**
 * Updates stats when a request is dropped due to bounded concurrency cap.
 */
export function recordDrop(stats) {
  return {
    ...stats,
    dropped: stats.dropped + 1,
  };
}

/**
 * Updates stats when a gateway request completes (or was aborted during fetch).
 */
export function recordResponse(stats, { res, isAborted, currentInFlight }) {
  if (isAborted) {
    return {
      ...stats,
      cancelled: stats.cancelled + 1,
      inFlight: currentInFlight,
    };
  }

  const parsed = parseCacheResponse(res.status, res.headers);
  const isHit = parsed.verdict === 'HIT';
  const isMiss = parsed.verdict === 'MISS';
  const isCoalesced = parsed.verdict === 'COALESCED';
  const isOther = parsed.verdict === 'STALE' || parsed.verdict === 'BYPASS' || parsed.verdict === 'BYPASS-AUTH';
  const isOk = res.ok && parsed.kind !== 'error';

  return {
    ...stats,
    completed: stats.completed + 1,
    succeeded: isOk ? stats.succeeded + 1 : stats.succeeded,
    failed: !isOk ? stats.failed + 1 : stats.failed,
    hits: isHit ? stats.hits + 1 : stats.hits,
    misses: isMiss ? stats.misses + 1 : stats.misses,
    coalesced: isCoalesced ? stats.coalesced + 1 : stats.coalesced,
    others: isOther ? stats.others + 1 : stats.others,
    inFlight: currentInFlight,
  };
}

/**
 * Updates stats when an unexpected network or runtime error occurs in fetch.
 */
export function recordError(stats, { err, isAborted, currentInFlight }) {
  const aborted = isAborted || (err && (err.name === 'AbortError' || err.message === 'The user aborted a request.'));
  return {
    ...stats,
    completed: aborted ? stats.completed : stats.completed + 1,
    cancelled: aborted ? stats.cancelled + 1 : stats.cancelled,
    failed: !aborted ? stats.failed + 1 : stats.failed,
    inFlight: currentInFlight,
  };
}

/**
 * Validates the core mathematical invariants of traffic accounting.
 * Returns { valid: true } or { valid: false, errors: string[] }.
 */
export function validateAccountingInvariants(stats) {
  const errors = [];
  const expectedDispatched = stats.completed + stats.cancelled + stats.inFlight;
  if (stats.dispatched !== expectedDispatched) {
    errors.push(`dispatched (${stats.dispatched}) !== completed (${stats.completed}) + cancelled (${stats.cancelled}) + inFlight (${stats.inFlight})`);
  }

  const expectedCompleted = stats.succeeded + stats.failed;
  if (stats.completed !== expectedCompleted) {
    errors.push(`completed (${stats.completed}) !== succeeded (${stats.succeeded}) + failed (${stats.failed})`);
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

