// `node --test ui/test` runs only this entry for a directory argument, so it loads every test file.
// Running a single file directly (node --test ui/test/render.test.mjs) also works.
await Promise.all([
  import('./contract.test.mjs'),
  import('./provenance.test.mjs'),
  import('./schema.test.mjs'),
  import('./connection.test.mjs'),
  import('./viewmodel.test.mjs'),
  import('./misc.test.mjs'),
  import('./forbidden.test.mjs'),
  import('./render.test.mjs'),
  import('./a11y.test.mjs'),
  import('./sim.test.mjs'),
  // Phantom Lab (G3)
  import('./lab.api.test.mjs'),
  import('./lab.stream.test.mjs'),
  import('./lab.view.test.mjs'),
  import('./lab.forms.test.mjs'),
  import('./lab.render.test.mjs'),
  import('./lab.guard.test.mjs'),
  import('./lab.fixes.test.mjs'),
  import('./lab.markdown.test.mjs'),
  import('./lab.hierarchy.test.mjs'),
  import('./lab.round2.test.mjs'),
  import('./tokens.test.mjs'),
  import('./shell.test.mjs'),
  // Functional UX logic (agent FLIB, registered by the lead)
  import('./func.result.test.mjs'),
  import('./func.link.test.mjs'),
  import('./func.gateway.test.mjs'),
  import('./func.progress.test.mjs'),
  import('./func.provenance.test.mjs'),
  // Design foundations v2 (agent V1, registered by the lead)
  import('./tokens.materials.test.mjs'),
  import('./motion.v2.test.mjs'),
  import('./shell.v2.test.mjs'),
  // Control Center Remediation & Regression Suites (WP1, WP3, WP4, WP5; PC-AUTO-030)
  import('./cacheStatus.test.mjs'),
  import('./hrw.test.mjs'),
  import('./historyGap.test.mjs'),
  import('./trafficGenerator.test.mjs'),
  // Cluster screen (batch B2, WP11)
  import('./cluster.test.mjs'),
  // Control Center sign-in (lane L2, D-045 item 2)
  import('./auth.test.mjs'),
  // Intelligence screen (lane L2, step 2)
  import('./intelligence.test.mjs'),
  // Start here, Demo Hub links, help lines, welcome (D-053)
  import('./starthere.test.mjs'),
]);
