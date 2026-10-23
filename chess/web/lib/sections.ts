export const SECTIONS = [
  { href: '/', label: 'Play & Analyze' },
  { href: '/lab', label: 'Training Lab' },
  { href: '/strength', label: 'Strength & Evaluation' },
  { href: '/games', label: 'Game Explorer' },
  { href: '/model', label: 'Model Inspector' },
  { href: '/runs', label: 'Experiments & Logs' },
];

export const PLACEHOLDERS: Record<string, { title: string; intro: string; shows: string[]; source: string }> = {
  lab: {
    title: 'Training Lab',
    intro: 'How each net was made. Vanta trains offline, so this is a history of runs, not a live monitor.',
    shows: ['Loss curve per training step', 'Learning-rate schedule, batch size, epochs, positions', 'Data generation runs: positions per second, how games ended'],
    source: 'train.log and datagen output, stored in Supabase',
  },
  strength: {
    title: 'Strength',
    intro: 'Whether each change made the engine stronger.',
    shows: ['Elo change per version, with error bars', 'Wins, draws and losses, split by colour', 'Rating anchor against limited Stockfish'],
    source: 'docs/results.md and the Rating workflow, stored in Supabase',
  },
  games: {
    title: 'Game Explorer',
    intro: 'Replay the games the engine played in its tests.',
    shows: ['Search by version, result and length', 'Replay with evaluation swings marked', 'Export to PGN'],
    source: 'games.pgn files from the SPRT workflow',
  },
  model: {
    title: 'Model Inspector',
    intro: 'What the network has learned.',
    shows: ['Version and parameter count', 'First-layer weights drawn as board heatmaps', 'Two nets compared on the same position'],
    source: 'the .nnue files in the repo, read in the browser',
  },
  runs: {
    title: 'Experiments',
    intro: 'Everything needed to reproduce a result.',
    shows: ['Tag, commit, net and data used for each run', 'Seeds and settings', 'Failed games and timeouts'],
    source: 'git tags and run records, stored in Supabase',
  },
};
