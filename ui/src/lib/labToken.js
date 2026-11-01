// Display-only reading of the role claim of a bearer token. The page never verifies a token (the gateway does), never
// stores or logs it, and shows nothing when the token cannot be read. The result only decides which hint is shown.

const OPERATE = ['scenario:operate', 'operator', 'admin'];
const OBSERVE = ['telemetry:read', 'observer', 'admin'];

function decodeBase64Url(s) {
  const b64 = String(s).replace(/-/g, '+').replace(/_/g, '/');
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4);
  const bin = typeof atob === 'function' ? atob(padded) : Buffer.from(padded, 'base64').toString('binary');
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

/** roles claim as an array of short strings, or null if the token is not a readable JWT with a roles array. */
export function tokenRoles(token) {
  try {
    const parts = String(token || '').split('.');
    if (parts.length !== 3) return null;
    const claims = JSON.parse(decodeBase64Url(parts[1]));
    if (!claims || !Array.isArray(claims.roles)) return null;
    return claims.roles.filter((r) => typeof r === 'string').map((r) => r.slice(0, 40)).slice(0, 12);
  } catch {
    return null;
  }
}

const has = (roles, list) => roles.some((r) => list.includes(r.toLowerCase()));

/** { canRead, canOperate } from a roles array, or null when roles are unknown. */
export function roleCapabilities(roles) {
  if (!Array.isArray(roles)) return null;
  return { canRead: has(roles, OBSERVE), canOperate: has(roles, OPERATE) };
}
