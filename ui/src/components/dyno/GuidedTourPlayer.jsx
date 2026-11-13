import React, { useState, useEffect } from 'react';
import Icon from '../Icon.jsx';

const TOUR_ACTS = [
  {
    act: 1,
    title: 'Act 1: Steady Baseline (2,500 req/s)',
    rps: 2500,
    narrative: 'Normal operating conditions. Requests are served in 0.4ms directly from local RAM cache. The database origin experiences zero latency spikes.',
  },
  {
    act: 2,
    title: 'Act 2: Flash Crowd Stampede (100,000 req/s)',
    rps: 100000,
    narrative: 'A flash sale or viral spike launches 100k requests/second. Without Phantom Cache, the database connection pool exhausts and crashes. With Phantom Cache, Edge RAM absorbs 99.8% of traffic instantly.',
  },
  {
    act: 3,
    title: 'Act 3: Singleflight Mutex Shield (50,000 req/s)',
    rps: 50000,
    narrative: 'A cache entry expires during a traffic storm. Rather than letting 50,000 requests slam the database simultaneously, the mutex coalesces them into 1 single upstream request while all other 49,999 wait.',
  },
];

/**
 * 3-Act Self-Driving Walkthrough Player.
 * Guides executive viewers through the core architecture in 30 seconds.
 */
export default function GuidedTourPlayer({ activeAct, onActSelect, onRpsChange }) {
  const [isPlaying, setIsPlaying] = useState(false);

  useEffect(() => {
    let timer;
    if (isPlaying) {
      timer = setInterval(() => {
        onActSelect((prev) => {
          const nextAct = prev >= 3 ? 1 : prev + 1;
          const config = TOUR_ACTS.find((a) => a.act === nextAct);
          if (config && onRpsChange) onRpsChange(config.rps);
          return nextAct;
        });
      }, 7500);
    }
    return () => clearInterval(timer);
  }, [isPlaying, onActSelect, onRpsChange]);

  const selectAct = (actNum) => {
    onActSelect(actNum);
    const config = TOUR_ACTS.find((a) => a.act === actNum);
    if (config && onRpsChange) onRpsChange(config.rps);
  };

  const currentConfig = TOUR_ACTS.find((a) => a.act === activeAct) || TOUR_ACTS[0];

  return (
    <div className="dyno-tour-strip" role="region" aria-label="Self-Driving Guided Tour">
      <div className="dyno-tour-strip__meta">
        <div className="dyno-tour-badge-row">
          <span className="dyno-tour-badge">SELF-DRIVING TOUR</span>
          <button
            type="button"
            className="btn btn--sm dyno-tour-play-btn"
            onClick={() => setIsPlaying((p) => !p)}
            aria-pressed={isPlaying}
          >
            <Icon name={isPlaying ? 'clock' : 'ok'} />
            <span>{isPlaying ? 'Pause Tour' : 'Auto-Play (30s Tour)'}</span>
          </button>
        </div>
        <p className="dyno-tour-text">{currentConfig.narrative}</p>
      </div>

      <div className="dyno-tour-strip__controls">
        {TOUR_ACTS.map((a) => (
          <button
            key={a.act}
            type="button"
            className={`btn btn--sm ${activeAct === a.act ? 'btn--primary' : ''}`}
            onClick={() => selectAct(a.act)}
            aria-pressed={activeAct === a.act}
          >
            {a.title.split(':')[0]}
          </button>
        ))}
      </div>
    </div>
  );
}
