import { Chess } from 'chess.js';

// White's point of view, in centipawns. `best` and `pv` are UCI moves like "e2e4" or "e7e8q".
export type Analysis = { best: string; cp: number; depth: number; nodes: number; pv: string[] };

export interface Engine {
  name: string;
  analyse(fen: string): Promise<Analysis | null>;
}

const VALUE: Record<string, number> = { p: 100, n: 300, b: 300, r: 500, q: 900 };

// Placeholder until the WASM build exists: random legal move, material count as the eval.
class StubEngine implements Engine {
  name = 'Stub engine (random move, material eval)';
  async analyse(fen: string): Promise<Analysis | null> {
    const g = new Chess(fen);
    const moves = g.moves({ verbose: true });
    if (!moves.length) return null;
    const m = moves[Math.floor(Math.random() * moves.length)];
    const uci = m.from + m.to + (m.promotion ?? '');
    let cp = 0;
    for (const row of g.board()) for (const sq of row) if (sq && sq.type !== 'k') cp += (sq.color === 'w' ? 1 : -1) * VALUE[sq.type];
    await new Promise((r) => setTimeout(r, 250));
    return { best: uci, cp, depth: 1, nodes: moves.length, pv: [uci] };
  }
}

export const engine: Engine = new StubEngine();
