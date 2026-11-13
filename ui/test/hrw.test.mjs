// ui/test/hrw.test.mjs
// Verification of Highest Random Weight (HRW) Rendezvous Hashing against authoritative Rust router.
// Consumes shared golden vectors from tests/vectors/hrw_vectors.json and executes 5,000-key cross-check.

import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { calculateHash, routeKey, toCanonicalKey } from '../src/lib/hrw.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const VECTORS_PATH = path.resolve(__dirname, '../../tests/vectors/hrw_vectors.json');

test('HRW canonical key transformation matches CacheKey::to_canonical_string()', () => {
  assert.equal(toCanonicalKey('/api/v1/cache/item-1'), '__default__:/item-1');
  assert.equal(toCanonicalKey('item-1'), '__default__:/item-1');
  assert.equal(toCanonicalKey('/catalog/product-492'), '__default__:/catalog/product-492');
  assert.equal(toCanonicalKey('/search?q=phantom'), '__default__:/search:?q=phantom');
  assert.equal(toCanonicalKey('/api/v1/cache/search?q=test&page=2'), '__default__:/search:?q=test&page=2');
  assert.equal(toCanonicalKey('/a?b?c'), '__default__:/a:?b?c', 'Preserves subsequent ? characters in query string (PC-AUTO-027)');
  assert.equal(toCanonicalKey('/api/v1/cache/users/42', 'tenantA'), 'tenantA:/users/42');
});

test('HRW 64-bit FNV-1a matches documented Rust golden vectors', () => {
  assert.equal(calculateHash('hotel_123', 1), 0x395fe85ed8f6c3d9n);
  assert.equal(calculateHash('hotel_123', 2), 0x395fe55ed8f6bec0n);
  assert.equal(calculateHash('user:1001', 1), 0xc08552b57b6f2d55n);
  assert.equal(calculateHash('', 0), 0xaf63bd4c8601b7dfn);
});

test('HRW implementation strictly satisfies all shared golden vectors in tests/vectors/hrw_vectors.json', () => {
  assert.ok(fs.existsSync(VECTORS_PATH), `Missing shared vectors file at ${VECTORS_PATH}`);
  const content = JSON.parse(fs.readFileSync(VECTORS_PATH, 'utf8'));
  const nodes = [{ node_id: 1 }, { node_id: 2 }, { node_id: 3 }];

  for (const vec of content.vectors) {
    for (const [nodeStr, hashObj] of Object.entries(vec.node_hashes)) {
      const nodeId = Number(nodeStr);
      const computedHash = calculateHash(vec.key, nodeId);
      assert.equal(
        computedHash.toString(),
        hashObj.dec,
        `Decimal hash mismatch for key="${vec.key}", nodeId=${nodeId}`
      );
      assert.equal(
        `0x${computedHash.toString(16).padStart(16, '0').toUpperCase()}`,
        hashObj.hex,
        `Hex hash mismatch for key="${vec.key}", nodeId=${nodeId}`
      );
    }

    const routing = routeKey(vec.key, nodes);
    assert.equal(
      routing.primary.node_id,
      vec.primary_node_id,
      `Primary owner mismatch for key="${vec.key}"`
    );
    assert.deepEqual(
      routing.replicas.map((n) => n.node_id),
      vec.replicas,
      `Replica order mismatch for key="${vec.key}"`
    );
  }
});

test('HRW 5,000-key cross-validation reaches 100% determinism and satisfies HRW monotonicity invariant', () => {
  const nodes3 = [{ node_id: 1 }, { node_id: 2 }, { node_id: 3 }];
  const nodes4 = [{ node_id: 1 }, { node_id: 2 }, { node_id: 3 }, { node_id: 4 }];
  let pseudoRand = 123456789;
  const lcg = () => {
    pseudoRand = (pseudoRand * 1664525 + 1013904223) >>> 0;
    return pseudoRand;
  };

  const keysCount = 5000;

  for (let i = 0; i < keysCount; i += 1) {
    const rawKey = `/api/v1/cache/item-${lcg() % 100000}`;
    const canonical = toCanonicalKey(rawKey);

    const r1 = routeKey(canonical, nodes3);
    const r2 = routeKey(canonical, nodes3);

    // Exact determinism: r1 === r2
    assert.equal(r1.primary.node_id, r2.primary.node_id);
    assert.deepEqual(
      r1.replicas.map((n) => n.node_id),
      r2.replicas.map((n) => n.node_id)
    );

    // HRW Monotonicity / Minimal Disruption Invariant:
    // When node 4 is added, the key must either remain with its old primary OR move to node 4.
    // It can never jump from node 1 to node 2 or node 3.
    const rWith4 = routeKey(canonical, nodes4);
    assert.ok(
      rWith4.primary.node_id === r1.primary.node_id || rWith4.primary.node_id === 4,
      `HRW invariant violated for key="${canonical}": old primary was ${r1.primary.node_id}, new primary is ${rWith4.primary.node_id}`
    );
  }
});
