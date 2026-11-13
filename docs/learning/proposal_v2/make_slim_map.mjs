// Derives layer_map_slim.json from layer_map.json (PROPOSAL, non-destructive). Slim = Part I with only the teaching core:
//  - per-chapter knowledge checks, codebase exploration tasks and challenge questions move to Part II (R7);
//  - Appendix H moves to Part II (R8);
//  - the "Ten hard questions to rehearse first" list of Appendix E comes to Part I;
//  - only 10 judge boxes stay in Part I (one core box per chapter, reused from layer_map.json key_judge_boxes, plus 2 more from Ch 8).
//   node docs/learning/proposal_v2/make_slim_map.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const HERE = dirname(fileURLToPath(import.meta.url));
const m = JSON.parse(readFileSync(join(HERE, 'layer_map.json'), 'utf8'));
const s = structuredClone(m);
for (const k of Object.keys(s.sections)) {
  if (/^ch\d\d:(knowledge-check|codebase-exploration-tasks|challenge-questions)$/.test(k)) { s.sections[k] = 'reference'; s.groups[k] = 'R7'; }
  if (k === 'appH' || k.startsWith('appH:')) { s.sections[k] = 'reference'; s.groups[k] = 'R8'; }
}
s.sections['appE:ten-hard-questions-to-rehearse-first'] = 'learning';
delete s.groups['appE:ten-hard-questions-to-rehearse-first'];
s.key_judge_boxes = ['ch01:1.4#1', 'ch02:2.2#1', 'ch03:3.5#1', 'ch04:4.5#1', 'ch05:5.6#2', 'ch06:6.9#2', 'ch07:7.9#1', 'ch08:8.13#1', 'ch08:8.13#2', 'ch08:8.13#7'];
s._meta = { ...m._meta, variant: 'slim', derived_from: 'layer_map.json', note: 'SLIM variant. ' + m._meta.note + ' Slim changes: study aids (knowledge check, exploration tasks, challenge questions) -> Part II R7; Appendix H -> Part II R8; appE "Ten hard questions" -> Part I; key_judge_boxes reduced to 10 (one core box per chapter + 2 more from Ch 8, all reused from the full map).' };
writeFileSync(join(HERE, 'layer_map_slim.json'), JSON.stringify(s, null, 1));
console.log('slim map written; reference sections:', Object.values(s.sections).filter((v) => v === 'reference').length, 'of', Object.keys(s.sections).length);
