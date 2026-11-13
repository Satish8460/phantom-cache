import { createHmac } from 'node:crypto';
import { writeFileSync } from 'node:fs';
const SECRET = '<redacted test secret>';
const b64 = (o) => Buffer.from(JSON.stringify(o)).toString('base64url');
function mint(claims) { const h = b64({ alg: 'HS256', typ: 'JWT' }), p = b64(claims); return `${h}.${p}.${createHmac('sha256', SECRET).update(`${h}.${p}`).digest('base64url')}`; }
const now = Math.floor(Date.now() / 1000);
const base = (roles, o = {}) => ({ sub: 'd7-' + roles.join('-'), iss: 'phantom-cache-auth', aud: 'phantom-cache-api', exp: now + 3600, iat: now, tenant_id: 'tenant-d7', roles, ...o });
const T = {
  observer: mint(base(['observer'])), operator: mint(base(['operator'])), admin: mint(base(['admin'])),
  expired: mint(base(['admin'], { exp: now - 3600 })), wrongaud: mint(base(['admin'], { aud: 'someone-else' })),
};
writeFileSync('D:/Temp/d7/tokens.json', JSON.stringify(T, null, 1));
console.log(Object.keys(T));
