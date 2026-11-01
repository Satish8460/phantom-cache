// ui/src/lib/hrw.js
// Highest Random Weight (HRW) Rendezvous Hashing algorithm matching src/dist/router.rs exactly.
// Note (PC-AUTO-027): Library-only component verified against Rust shared test vectors; integrated into WP11 Cluster View.

const FNV_OFFSET_BASIS = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const MASK_64 = 0xffffffffffffffffn;

const encoder = new TextEncoder();

/**
 * Computes 64-bit FNV-1a hash matching `RendezvousRouter::calculate_hash` in `src/dist/router.rs`.
 * Input bytes: UTF-8 bytes of canonical key followed by single `node_id` byte (0-255).
 *
 * @param {string} canonicalKey - UTF-8 canonical key (e.g. "__default__:/catalog/item-1")
 * @param {number} nodeId - Node ID byte (0-255)
 * @returns {bigint} 64-bit unsigned BigInt
 */
export function calculateHash(canonicalKey, nodeId) {
  let hash = FNV_OFFSET_BASIS;
  const bytes = encoder.encode(canonicalKey);
  for (let i = 0; i < bytes.length; i += 1) {
    hash ^= BigInt(bytes[i]);
    hash = (hash * FNV_PRIME) & MASK_64;
  }
  hash ^= BigInt(nodeId & 0xff);
  hash = (hash * FNV_PRIME) & MASK_64;
  return hash;
}

/**
 * Converts user-entered path or key to canonical string format matching `CacheKey::to_canonical_string()`.
 * Strips `/api/v1/cache` prefix if present; ensures leading slash; prepends tenant `__default__:`.
 *
 * @param {string} input - User-entered key or path
 * @param {string} tenant - Default '__default__'
 * @returns {string} Canonical key string
 */
export function toCanonicalKey(input, tenant = '__default__') {
  let clean = String(input || '').trim();
  if (clean.startsWith('/api/v1/cache')) {
    clean = clean.slice('/api/v1/cache'.length);
  }
  if (!clean.startsWith('/')) {
    clean = `/${clean}`;
  }
  const qIdx = clean.indexOf('?');
  if (qIdx !== -1) {
    const pathPart = clean.slice(0, qIdx);
    const queryPart = clean.slice(qIdx + 1);
    return `${tenant}:${pathPart}:?${queryPart}`;
  }
  return `${tenant}:${clean}`;
}

/**
 * Routes a key across a list of nodes using HRW rendezvous hashing.
 * Matches `RendezvousRouter::route_key` and `get_replicas` in `src/dist/router.rs`.
 *
 * @param {string} canonicalKey - Canonical key string
 * @param {Array<{node_id: number, [key: string]: any}>} nodes - Node list
 * @returns {{
 *   primary: object|null,
 *   replicas: Array<object>,
 *   scores: Array<{node: object, hash: bigint, hex: string, weight: number}>
 * }}
 */
export function routeKey(canonicalKey, nodes) {
  if (!nodes || nodes.length === 0) {
    return { primary: null, replicas: [], scores: [] };
  }

  const scored = nodes.map((node) => {
    const hash = calculateHash(canonicalKey, node.node_id);
    const weight = Number(hash);
    return {
      node,
      hash,
      hex: `0x${hash.toString(16).padStart(16, '0').toUpperCase()}`,
      weight,
    };
  });

  // Sort descending by weight (hash as f64). Strict `>` preserves first-node win on exact ties.
  scored.sort((a, b) => {
    if (b.weight > a.weight) return 1;
    if (b.weight < a.weight) return -1;
    return 0;
  });

  const primary = scored.length > 0 ? scored[0].node : null;
  const replicas = scored.slice(1).map((s) => s.node);

  return {
    primary,
    replicas,
    scores: scored,
  };
}
