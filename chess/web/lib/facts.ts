// Real results from docs/results.md (all at 2+0.02, each against the previous version).
export const VERSIONS = [
  { tag: 'v0.12', change: 'Reverse futility pruning', elo: 73.2, err: 24.9, w: 179, d: 142, l: 93, status: 'Current' },
  { tag: 'v0.11', change: 'Net vanta-v4 (128M positions)', elo: 27.8, err: 16.0, w: 520, d: 355, l: 417, status: 'Archived' },
  { tag: 'v0.10', change: 'Net vanta-v3 (88M positions)', elo: 109.1, err: 34.6, w: 182, d: 82, l: 78, status: 'Archived' },
  { tag: 'v0.9', change: 'AVX2 build', elo: 91.9, err: 27.9, w: 153, d: 137, l: 62, status: 'Archived' },
  { tag: 'v0.8', change: 'NNUE, net vanta-v2 (48M positions)', elo: 238.1, err: 56.1, w: 147, d: 25, l: 28, status: 'Archived' },
];
// Nodes to reach depth 14 from the start position (single thread).
export const DEPTH14 = [
  { label: 'v0.11', nodes: 4.47 },
  { label: '+ PVS', nodes: 4.09 },
  { label: '+ RFP (v0.12)', nodes: 2.11 },
];
