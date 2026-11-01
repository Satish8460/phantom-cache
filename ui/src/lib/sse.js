// Incremental parser for text/event-stream (WHATWG HTML standard, "Server-sent events"), written for fetch
// streaming because EventSource cannot send an Authorization header (docs/design/sse_auth_options.md, option D).
//
// push(chunk) accepts a string of any size (chunk boundaries may fall anywhere, including inside a field name,
// between CR and LF, or inside a multi-byte sequence that the caller's TextDecoder has already joined) and returns
// the items completed by that chunk:
//   { type: 'event', id, event, data, retry }   dispatched when a blank line ends an event that has data
//   { type: 'comment', text }                    a line starting with ':' (the gateway sends `:heartbeat`)
// Lines end with CRLF, LF or CR. A leading BOM is ignored. `id` values containing NUL are ignored (per standard).

export function createSseParser() {
  let buf = '';
  let started = false;
  let lastEventId = '';
  let data = [];
  let hasData = false;
  let eventType = '';
  let retry = null;

  function dispatch(out) {
    if (hasData) {
      out.push({ type: 'event', id: lastEventId, event: eventType || 'message', data: data.join('\n'), retry });
    }
    data = [];
    hasData = false;
    eventType = '';
    retry = null;
  }

  function handleLine(line, out) {
    if (line === '') {
      dispatch(out);
      return;
    }
    if (line.charCodeAt(0) === 0x3a) {
      out.push({ type: 'comment', text: line.slice(1).replace(/^ /, '') });
      return;
    }
    const idx = line.indexOf(':');
    let field;
    let value;
    if (idx === -1) {
      field = line;
      value = '';
    } else {
      field = line.slice(0, idx);
      value = line.slice(idx + 1);
      if (value.startsWith(' ')) value = value.slice(1);
    }
    switch (field) {
      case 'event':
        eventType = value;
        break;
      case 'data':
        data.push(value);
        hasData = true;
        break;
      case 'id':
        if (!value.includes('\u0000')) lastEventId = value;
        break;
      case 'retry':
        if (/^[0-9]+$/.test(value)) retry = Number(value);
        break;
      default:
        break;
    }
  }

  return {
    push(chunk) {
      const out = [];
      buf += chunk;
      if (!started && buf.length > 0) {
        started = true;
        if (buf.charCodeAt(0) === 0xfeff) buf = buf.slice(1);
      }
      for (;;) {
        const m = /\r\n|\n|\r/.exec(buf);
        if (!m) break;
        // A CR at the very end of the buffer may be the first half of a CRLF split across chunks: wait.
        if (m[0] === '\r' && m.index === buf.length - 1) break;
        const line = buf.slice(0, m.index);
        buf = buf.slice(m.index + m[0].length);
        handleLine(line, out);
      }
      return out;
    },
    /** Bytes of an incomplete trailing line currently buffered (for diagnostics/tests). */
    pending() {
      return buf.length;
    },
    /** The last `id:` value seen, even if its event is not complete yet. */
    lastEventId() {
      return lastEventId;
    },
  };
}

/** Parse the JSON `data` of a dispatched event. Returns { ok, value } and never throws. */
export function parseEventData(ev) {
  try {
    return { ok: true, value: JSON.parse(ev.data) };
  } catch {
    return { ok: false, value: null };
  }
}

/** Convenience for tests and replays: parse a whole document at once. */
export function parseSseText(text) {
  const p = createSseParser();
  return p.push(text).filter((x) => x.type === 'event');
}
