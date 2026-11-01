import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { pushPoint, toSegments, MAX_SAMPLE_GAP_MS } from '../src/lib/history.js';

describe('historyGap - PC-AUTO-024 gap insertion and segment splitting', () => {
  it('inserts a null point when sample gap exceeds MAX_SAMPLE_GAP_MS', () => {
    let series = [];
    const t0 = 1000;
    // Push regular point at t0
    series = pushPoint(series, t0, { v: 42, src: 'measured' });
    assert.equal(series.length, 1);
    assert.deepEqual(series[0], { t: 1000, v: 42 });

    // Push next point within normal interval (1s later)
    series = pushPoint(series, t0 + 1000, { v: 45, src: 'measured' });
    assert.equal(series.length, 2);
    assert.deepEqual(series[1], { t: 2000, v: 45 });

    // Simulate page reload or 20s idle gap (gap = 20,000ms > MAX_SAMPLE_GAP_MS 3,000ms)
    const tGap = t0 + 1000 + 20000;
    series = pushPoint(series, tGap, { v: 50, src: 'measured' });
    // Expect 4 points now: original 2 + null gap point + new point
    assert.equal(series.length, 4);
    assert.deepEqual(series[2], { t: 2001, v: null }, 'Null gap marker must be inserted at gap start');
    assert.deepEqual(series[3], { t: tGap, v: 50 }, 'New point appended after gap marker');
  });

  it('toSegments breaks line into multiple segments across null gap point', () => {
    let series = [];
    series = pushPoint(series, 1000, { v: 10, src: 'measured' });
    series = pushPoint(series, 2000, { v: 20, src: 'measured' });
    // 10 second gap
    series = pushPoint(series, 12000, { v: 30, src: 'measured' });
    series = pushPoint(series, 13000, { v: 40, src: 'measured' });

    assert.equal(series.length, 5); // 2 + 1 null + 2
    const { segments, min, max } = toSegments(series, 100, 50);

    assert.equal(segments.length, 2, 'Line must be split into exactly 2 disconnected segments');
    assert.equal(segments[0].length, 2, 'First segment has 2 points');
    assert.equal(segments[1].length, 2, 'Second segment has 2 points');
    assert.equal(min, 10);
    assert.equal(max, 40);
  });
});
