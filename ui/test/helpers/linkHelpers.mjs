import { connectionReducer } from '../../src/lib/connection.js';
import { livePayload } from './fixtures.mjs';

export const frame = (p = livePayload()) => JSON.stringify(p);
export const msg = (state, text, now) => connectionReducer(state, { type: 'message', text, now });
