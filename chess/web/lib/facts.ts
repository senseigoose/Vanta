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

// Early history (from the changelog): gains of each version over the one before, at 2+0.02.
export const HISTORY = [
  { tag: 'v0.2', change: 'Transposition table', elo: 112 },
  { tag: 'v0.3', change: 'Faster sliders and move lists', elo: 17 },
  { tag: 'v0.4', change: 'Null-move pruning', elo: 57 },
  { tag: 'v0.5', change: 'Killers and history', elo: 71 },
  { tag: 'v0.6', change: 'Late-move reductions', elo: 33 },
  { tag: 'v0.7', change: 'Mailbox board, pin-aware legality', elo: 171 },
];

// v0.11 against Stockfish at limited strength, 10+0.1, 100 games per level.
export const ANCHOR = [
  { level: 2400, score: 51.0, rating: 2407, err: 59 },
  { level: 2700, score: 17.0, rating: 2425, err: 82 },
  { level: 3000, score: 7.5, rating: 2564, err: 126 },
];

// Training loss per superbatch (one pass over the data each).
export const LOSS_V1 = [0.011449, 0.009158, 0.008731, 0.008462, 0.008276, 0.008145, 0.008045, 0.007968, 0.007905, 0.007852, 0.007806, 0.007767, 0.007733, 0.007702, 0.007675, 0.007506, 0.007488, 0.00748, 0.007475, 0.007471, 0.007467, 0.007463, 0.00746, 0.007457, 0.007454, 0.007451, 0.007448, 0.007445, 0.007442, 0.007439, 0.007417, 0.007415, 0.007415, 0.007414, 0.007414, 0.007414, 0.007413, 0.007413, 0.007413, 0.007412];
export const LOSS_V2 = { xs: [1, 2, 3, 5, 10, 15, 20, 25, 30, 35, 38, 39, 40], ys: [0.009941, 0.009269, 0.009148, 0.009029, 0.008908, 0.008863, 0.00874, 0.008734, 0.008729, 0.008713, 0.008713, 0.008713, 0.008713] };

export const NETS = [
  { id: 'vanta-v1', positions: '8M', shards: '1–8', loss: '0.007412', note: 'Level with the handcrafted eval per node' },
  { id: 'vanta-v2', positions: '48M', shards: '1–48', loss: '0.008713', note: '+395 Elo per node vs handcrafted eval' },
  { id: 'vanta-v3', positions: '88M', shards: '1–88', loss: 'not recorded', note: '+109 Elo over v2' },
  { id: 'vanta-v4', positions: '128M', shards: '1–88, 93–132', loss: 'not recorded', note: '+28 Elo over v3' },
];

// How games ended in two 30,000-position self-play runs (same seed, 5000 nodes per move).
export const GAME_ENDS = [
  ['Checkmate', 8.5, 2.3], ['Stalemate', 0.3, 0.5], ['50-move rule', 1.7, 5.4], ['Repetition', 20.1, 16.8],
  ['Insufficient material', 5.1, 5.7], ['Move cap', 0.3, 0.0], ['Adjudicated win', 64.1, 69.3],
] as const;

export const EXPERIMENTS = [
  { what: 'Futility margin 60 and 150 (two runs, order not recorded)', vs: 'margin 100', elo: -4.9, err: 9.4, games: 6000, outcome: 'Inconclusive', ref: 'untagged' },
  { what: 'Futility pruning (depth ≤ 4, margin 100)', vs: 'v0.12', elo: 6.7, err: 9.6, games: 3000, outcome: 'Inconclusive', ref: 'untagged' },
  { what: 'RFP margin 140 (UCI option)', vs: 'margin 100', elo: -6.5, err: 9.4, games: 3000, outcome: 'Inconclusive', ref: 'untagged' },
  { what: 'RFP margin 70 (UCI option)', vs: 'margin 100', elo: 4.2, err: 9.6, games: 3000, outcome: 'Inconclusive', ref: 'untagged' },
  { what: 'v0.12 Reverse futility pruning', vs: 'PVS commit', elo: 73.2, err: 24.9, games: 414, outcome: 'H1 accepted', ref: '0a27680' },
  { what: 'Principal-variation search', vs: 'v0.11', elo: -3.5, err: 9.3, games: 3000, outcome: 'Inconclusive', ref: 'c77f669' },
  { what: 'v0.11 Net vanta-v4 (128M positions)', vs: 'v0.10', elo: 27.8, err: 16.0, games: 1292, outcome: 'H1 accepted', ref: 'a8ec660' },
  { what: 'v0.10 Net vanta-v3 (88M positions)', vs: 'v0.9', elo: 109.1, err: 34.6, games: 342, outcome: 'H1 accepted', ref: '09a3ee1' },
  { what: 'v0.9 AVX2 build', vs: 'v0.8', elo: 91.9, err: 27.9, games: 352, outcome: 'H1 accepted', ref: '689ad30' },
  { what: 'v0.8 NNUE, net vanta-v2 (48M positions)', vs: 'v0.7', elo: 238.1, err: 56.1, games: 200, outcome: 'H1 accepted', ref: '903779c' },
  { what: 'Net vanta-v2 vs same build, handcrafted eval, 50,000 nodes', vs: 'handcrafted', elo: 394.9, err: 95.1, games: 166, outcome: 'H1 accepted', ref: 'diagnostic' },
  { what: 'Net vanta-v1 vs same build, handcrafted eval, 50,000 nodes', vs: 'handcrafted', elo: -2.8, err: 11.1, games: 3000, outcome: 'Inconclusive', ref: 'diagnostic' },
  { what: 'NNUE with net vanta-v1', vs: 'v0.7', elo: -66.9, err: 28.9, games: 468, outcome: 'H0 accepted', ref: 'untagged' },
];
