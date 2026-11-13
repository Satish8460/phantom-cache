import test from 'node:test';
import assert from 'node:assert';

function validateSchema2Contract(payload) {
  assert.strictEqual(payload.schema, 2, 'Payload must declare schema: 2');

  if (payload.mode === 'live') {
    // Recursive check: zero fields may carry src: "simulated"
    function checkNode(node, path) {
      if (!node || typeof node !== 'object') return;

      if (Array.isArray(node)) {
        node.forEach((item, idx) => checkNode(item, `${path}[${idx}]`));
      } else {
        if (node.src === 'simulated') {
          throw new Error(`Contract violation at '${path}': found 'src: "simulated"' in live mode`);
        }
        if (node.src === 'unavailable' && Object.prototype.hasOwnProperty.call(node, 'v')) {
          throw new Error(`Contract violation at '${path}': unavailable field must not carry a 'v' property`);
        }
        for (const [key, val] of Object.entries(node)) {
          checkNode(val, `${path}.${key}`);
        }
      }
    }

    checkNode(payload, 'root');

    // Live mode must not contain counterfactual economics fields
    assert.strictEqual(payload.avoided_compute_dollars, undefined, 'avoided_compute_dollars must be absent in live mode');
    assert.strictEqual(payload.backend_load_ratio, undefined, 'backend_load_ratio must be absent in live mode');
    assert.strictEqual(payload.cost, undefined, 'cost must be absent in live mode');
  }

  return true;
}

test('valid live schema 2 payload passes contract validation', () => {
  const validPayload = {
    schema: 2,
    mode: 'live',
    as_of_ms: 1730150000000,
    uptime_ms: 60000,
    capabilities: {
      cluster: 'disabled',
      controller: 'none',
      origin_metrics: 'available'
    },
    metrics: {
      requests_total: { src: 'measured', v: 500, unit: 'count' },
      cache_hits: { src: 'measured', v: 450, unit: 'count' },
      cache_misses: { src: 'measured', v: 50, unit: 'count' },
      hit_ratio_cumulative: { src: 'derived', v: 0.9, unit: 'ratio' },
      bytes_resident: { src: 'unavailable', reason: 'not_implemented', unit: 'bytes' },
      latency_ms: { src: 'measured', unit: 'ms', p50: 1, p90: 2, p95: 5, p99: 10, samples: 500 },
      origin: {
        requests_started: { src: 'measured', v: 50, unit: 'count' },
        requests_completed_ok: { src: 'measured', v: 50, unit: 'count' },
        in_flight: { src: 'measured', v: 0, unit: 'count' }
      }
    },
    hot_keys: { src: 'unavailable', reason: 'not_implemented' },
    nodes: [
      { node_id: 1, state: 'Active', state_src: 'simulated', cpu: { src: 'unavailable', reason: 'not_measured' } }
    ]
  };

  assert.doesNotThrow(() => validateSchema2Contract(validPayload));
});

test('live payload with simulated field fails contract validation', () => {
  const poisonedPayload = {
    schema: 2,
    mode: 'live',
    metrics: {
      requests_total: { src: 'measured', v: 100 },
      backend_load: { src: 'simulated', v: 0.25 } // FORBIDDEN IN LIVE
    }
  };

  assert.throws(
    () => validateSchema2Contract(poisonedPayload),
    /Contract violation.*found 'src: "simulated"' in live mode/
  );
});

test('unavailable field with v property fails contract validation', () => {
  const invalidUnavailable = {
    schema: 2,
    mode: 'live',
    metrics: {
      bytes_resident: { src: 'unavailable', v: 0, reason: 'not_implemented' } // FORBIDDEN 'v'
    }
  };

  assert.throws(
    () => validateSchema2Contract(invalidUnavailable),
    /unavailable field must not carry a 'v' property/
  );
});

test('simulation mode allows simulated fields', () => {
  const simPayload = {
    schema: 2,
    mode: 'simulation',
    avoided_compute_dollars: 0.05,
    metrics: {
      backend_load: { src: 'simulated', v: 0.25 }
    }
  };

  assert.doesNotThrow(() => validateSchema2Contract(simPayload));
});

