import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  createTrafficStats,
  recordDispatch,
  recordDrop,
  recordResponse,
  recordError,
  validateAccountingInvariants,
} from '../src/lib/trafficAccounting.js';

describe('TrafficGenerator Accounting Invariants (PC-AUTO-023, PC-AUTO-030)', () => {
  it('strictly preserves dispatched = completed + cancelled + inFlight and completed = succeeded + failed across mix of hits, misses, errors, and aborts', async () => {
    const MAX_IN_FLIGHT = 6;
    let inFlightCount = 0;
    let stats = createTrafficStats();

    const activeControllers = new Set();

    // Simulated fetcher that can succeed, fail, or be aborted
    const fakeExecute = async (url, path, { signal, behavior }) => {
      if (signal.aborted) {
        return { ok: false, status: 0, headers: new Headers(), bodyText: 'aborted', error: { kind: 'aborted' } };
      }
      if (behavior === 'hit') {
        const h = new Headers();
        h.set('x-cache-status', 'HIT');
        return { ok: true, status: 200, headers: h, bodyText: 'hit-body' };
      }
      if (behavior === 'miss') {
        const h = new Headers();
        h.set('x-cache-status', 'MISS');
        return { ok: true, status: 200, headers: h, bodyText: 'miss-body' };
      }
      if (behavior === 'coalesced') {
        const h = new Headers();
        h.set('x-cache-status', 'COALESCED');
        return { ok: true, status: 200, headers: h, bodyText: 'coalesced-body' };
      }
      if (behavior === 'bypass') {
        const h = new Headers();
        h.set('x-cache-status', 'BYPASS');
        return { ok: true, status: 200, headers: h, bodyText: 'bypass-body' };
      }
      if (behavior === 'server_error') {
        return { ok: false, status: 500, headers: new Headers(), bodyText: 'Internal Error', error: { kind: 'server' } };
      }
      if (behavior === 'slow_abort') {
        return new Promise((resolve) => {
          const onAbort = () => {
            resolve({ ok: false, status: 0, headers: new Headers(), bodyText: 'aborted', error: { kind: 'aborted' } });
          };
          signal.addEventListener('abort', onAbort, { once: true });
        });
      }
      throw new Error('Unknown behavior');
    };

    const fireRequest = async (behavior) => {
      if (inFlightCount >= MAX_IN_FLIGHT) {
        stats = recordDrop(stats);
        return;
      }
      inFlightCount += 1;
      stats = recordDispatch(stats, inFlightCount);

      const controller = new AbortController();
      activeControllers.add(controller);

      try {
        const res = await fakeExecute('http://127.0.0.1:8085', '/test', {
          signal: controller.signal,
          behavior,
        });

        inFlightCount = Math.max(0, inFlightCount - 1);
        const isAborted = controller.signal.aborted || res?.error?.kind === 'aborted';

        stats = recordResponse(stats, {
          res,
          isAborted,
          currentInFlight: inFlightCount,
        });
      } catch (err) {
        inFlightCount = Math.max(0, inFlightCount - 1);
        const isAborted = err.name === 'AbortError' || controller.signal.aborted;
        stats = recordError(stats, {
          err,
          isAborted,
          currentInFlight: inFlightCount,
        });
      } finally {
        activeControllers.delete(controller);
      }
    };

    // 1. Fire sequence of normal requests
    await fireRequest('hit');
    await fireRequest('miss');
    await fireRequest('coalesced');
    await fireRequest('bypass');
    await fireRequest('server_error');

    // Verify invariants mid-flight
    assert.equal(stats.dispatched, 5);
    assert.equal(stats.completed, 5);
    assert.equal(stats.succeeded, 4);
    assert.equal(stats.failed, 1);
    assert.equal(stats.cancelled, 0);
    assert.equal(stats.inFlight, 0);
    assert.equal(stats.dispatched, stats.completed + stats.cancelled + stats.inFlight);
    assert.equal(stats.completed, stats.succeeded + stats.failed);
    assert.equal(stats.hits + stats.misses + stats.coalesced + stats.others, stats.succeeded);
    assert.equal(validateAccountingInvariants(stats).valid, true);

    // 2. Launch 4 slow requests in flight
    const p1 = fireRequest('slow_abort');
    const p2 = fireRequest('slow_abort');
    const p3 = fireRequest('slow_abort');
    const p4 = fireRequest('slow_abort');

    assert.equal(stats.dispatched, 9);
    assert.equal(stats.inFlight, 4);
    assert.equal(stats.dispatched, stats.completed + stats.cancelled + stats.inFlight);
    assert.equal(validateAccountingInvariants(stats).valid, true);

    // 3. Try to exceed MAX_IN_FLIGHT (fire 3 more: 2 admitted to reach 6, 1 dropped by backpressure)
    const p5 = fireRequest('slow_abort');
    const p6 = fireRequest('slow_abort');
    const p7 = fireRequest('slow_abort'); // should be dropped

    assert.equal(stats.dropped, 1, '7th concurrent request must be dropped by backpressure cap');
    assert.equal(stats.inFlight, 6, 'inFlight must never exceed MAX_IN_FLIGHT (6)');
    assert.equal(stats.dispatched, 11);
    assert.equal(validateAccountingInvariants(stats).valid, true);

    // 4. Simulate stopAll(): abort active controllers
    activeControllers.forEach((c) => c.abort());
    activeControllers.clear();

    await Promise.all([p1, p2, p3, p4, p5, p6, p7]);

    // 5. Final invariant verification after stop
    assert.equal(stats.inFlight, 0, 'inFlight must settle to 0 after aborting');
    assert.equal(stats.cancelled, 6, 'All 6 aborted in-flight requests must be counted in cancelled');
    assert.equal(stats.completed, 5, 'Completed count must NOT be incremented by aborted requests');
    assert.equal(stats.dispatched, stats.completed + stats.cancelled + stats.inFlight, 'dispatched === completed + cancelled + inFlight');
    assert.equal(stats.completed, stats.succeeded + stats.failed, 'completed === succeeded + failed');
    assert.equal(validateAccountingInvariants(stats).valid, true);

    // Displayed tiles sum: completed + cancelled + inFlight === dispatched
    const displayedSum = stats.hits + stats.misses + stats.coalesced + stats.others + stats.failed + stats.cancelled + stats.inFlight;
    assert.equal(displayedSum, stats.dispatched, 'All displayed outcome tiles must sum to dispatched');
  });
});

