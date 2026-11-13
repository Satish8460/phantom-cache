// scripts/lib/jwt_mint.mjs
//
// DEV-ONLY helper: mints and (for display) decodes HS256 JWTs with Node `crypto`; no dependency. The claim names are
// the ones the gateway reads (src/security/jwt.rs:31-42: sub, iss, aud, exp, iat, tenant_id, roles; roles
// `cache:read`, `cache:invalidate`, `telemetry:read`, `scenario:operate`, `admin`).
// `decodeClaimsForDisplay` does NOT verify the signature: it must only be used to show who a token claims to be.

import { createHmac, randomBytes } from 'node:crypto';

const b64u = (buf) => Buffer.from(buf).toString('base64url');

export function newSecret(bytes = 48) { return randomBytes(bytes).toString('hex'); }

export function mintHS256(claims, secret) {
  const head = b64u(JSON.stringify({ alg: 'HS256', typ: 'JWT' }));
  const body = b64u(JSON.stringify(claims));
  const sig = createHmac('sha256', secret).update(`${head}.${body}`).digest('base64url');
  return `${head}.${body}.${sig}`;
}

/** Display-only decoding. Returns the claims object or null. Never verifies. */
export function decodeClaimsForDisplay(token) {
  try {
    const parts = String(token).split('.');
    if (parts.length !== 3) return null;
    return JSON.parse(Buffer.from(parts[1], 'base64url').toString('utf8'));
  } catch { return null; }
}

const FULL = ['cache:read', 'cache:invalidate', 'telemetry:read'];

/** The demo users. Returns { key: { label, description, token, claims } }. */
export function mintDemoTokens({ secret, issuer, audience, now = Math.floor(Date.now() / 1000), ttlSec = 86400 }) {
  const mk = (sub, tenant, roles, over = {}) => {
    const claims = { sub, iss: issuer, aud: audience, exp: now + ttlSec, iat: now, tenant_id: tenant, roles, ...over };
    return { claims, token: mintHS256(claims, secret) };
  };
  const users = {
    tenantA: { label: 'Tenant A (alice)', description: 'cache:read, cache:invalidate, telemetry:read; tenant tenant-a', ...mk('demo-alice', 'tenant-a', FULL) },
    tenantB: { label: 'Tenant B (bob)', description: 'cache:read, cache:invalidate, telemetry:read; tenant tenant-b', ...mk('demo-bob', 'tenant-b', FULL) },
    readonly: { label: 'Read-only (carol, tenant A)', description: 'cache:read, telemetry:read; tenant tenant-a; cannot DELETE', ...mk('demo-carol-readonly', 'tenant-a', ['cache:read', 'telemetry:read']) },
    expired: { label: 'Expired token (tenant A)', description: 'valid signature, exp one hour in the past; must be refused 401', ...mk('demo-expired', 'tenant-a', FULL, { exp: now - 3600, iat: now - 7200 }) },
    wrongAudience: { label: 'Wrong audience (tenant A)', description: 'valid signature, aud is not this gateway; must be refused 401', ...mk('demo-wrong-aud', 'tenant-a', FULL, { aud: 'some-other-api' }) },
  };
  return users;
}
