import { Chess } from 'chess.js';

// White's point of view, in centipawns. `best` and `pv` are UCI moves like "e2e4" or "e7e8q".
export type Analysis = { best: string; cp: number; depth: number; nodes: number; pv: string[] };

// `elo` (Stockfish only): play at about this strength. Leave it out for full strength.
export type SearchOpts = { movetime?: number; elo?: number };

export interface Engine {
  name: string;
  analyse(fen: string, opts?: SearchOpts): Promise<Analysis | null>;
}

export type EngineId = 'stockfish' | 'vanta' | 'stub';
export const ENGINES: { id: EngineId; label: string }[] = [
  { id: 'stockfish', label: 'Stockfish (WASM)' },
  { id: 'vanta', label: 'Vanta (WASM)' },
  { id: 'stub', label: 'Stub (random move)' },
];

const VALUE: Record<string, number> = { p: 100, n: 300, b: 300, r: 500, q: 900 };

// Placeholder until the Vanta WASM build exists: random legal move, material count as the eval.
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

// Wait for a line starting with `prefix` from a UCI worker.
function waitFor(w: Worker, prefix: string, ms = 30000): Promise<void> {
  return new Promise((resolve, reject) => {
    const done = (fn: () => void) => {
      clearTimeout(timer);
      w.removeEventListener('message', onMsg);
      w.removeEventListener('error', onErr);
      fn();
    };
    const onMsg = (e: MessageEvent) => {
      if (typeof e.data === 'string' && e.data.startsWith(prefix)) done(resolve);
    };
    const onErr = () => done(() => reject(new Error('The Stockfish worker failed to start.')));
    const timer = setTimeout(() => done(() => reject(new Error('Stockfish did not answer in time.'))), ms);
    w.addEventListener('message', onMsg);
    w.addEventListener('error', onErr);
  });
}

// Stockfish compiled to WebAssembly, run in a Web Worker and driven over UCI.
// Files live in public/stockfish/ (fetched by scripts/get-stockfish.sh); manifest.json names the loader.
class StockfishEngine implements Engine {
  name = 'Stockfish (WASM, single thread)';
  private ready: Promise<Worker> | null = null;
  private chain: Promise<unknown> = Promise.resolve();
  private seq = 0;
  private searching: Worker | null = null;

  private start(): Promise<Worker> {
    if (!this.ready) {
      this.ready = (async () => {
        const res = await fetch('/stockfish/manifest.json');
        if (!res.ok) throw new Error('Stockfish files are missing. Run scripts/get-stockfish.sh in chess/web and commit public/stockfish.');
        const man = (await res.json()) as { js: string; name?: string };
        if (man.name) this.name = man.name;
        const w = new Worker('/stockfish/' + man.js);
        w.postMessage('uci');
        await waitFor(w, 'uciok');
        w.postMessage('isready');
        await waitFor(w, 'readyok');
        return w;
      })();
      this.ready.catch(() => { this.ready = null; }); // allow a retry after a failure
    }
    return this.ready;
  }

  analyse(fen: string, opts: SearchOpts = {}): Promise<Analysis | null> {
    const mine = ++this.seq;
    this.searching?.postMessage('stop'); // let the running search finish quickly
    const run = this.chain.then(() => this.search(fen, opts, mine));
    this.chain = run.catch(() => null);
    return run;
  }

  private async search(fen: string, opts: SearchOpts, mine: number): Promise<Analysis | null> {
    if (mine !== this.seq) return null; // a newer request replaced this one while it waited
    const w = await this.start();
    if (mine !== this.seq) return null;
    const sign = fen.split(' ')[1] === 'b' ? -1 : 1; // UCI scores are for the side to move
    let last: Analysis | null = null;
    return new Promise<Analysis | null>((resolve) => {
      const onMsg = (e: MessageEvent) => {
        const line = typeof e.data === 'string' ? e.data : '';
        if (line.startsWith('info ') && line.includes(' pv ')) {
          const t = line.split(' ');
          const at = (k: string) => t.indexOf(k);
          const num = (k: string) => (at(k) >= 0 ? Number(t[at(k) + 1]) : NaN);
          const si = at('score');
          if (si < 0) return;
          const v = Number(t[si + 2]);
          const stm = t[si + 1] === 'mate' ? (v > 0 ? 10000 - v : -10000 - v) : v;
          const pv = t.slice(at('pv') + 1);
          if (Number.isNaN(stm) || !pv.length) return;
          last = { best: pv[0], cp: sign * stm, depth: num('depth') || 0, nodes: num('nodes') || 0, pv };
        } else if (line.startsWith('bestmove')) {
          w.removeEventListener('message', onMsg);
          this.searching = null;
          const best = line.split(' ')[1];
          if (!best || best === '(none)') return resolve(null);
          resolve(last ? { ...last, best } : { best, cp: 0, depth: 0, nodes: 0, pv: [best] });
        }
      };
      w.addEventListener('message', onMsg);
      this.searching = w;
      const limited = opts.elo !== undefined;
      w.postMessage(`setoption name UCI_LimitStrength value ${limited}`);
      if (limited) w.postMessage(`setoption name UCI_Elo value ${opts.elo}`);
      w.postMessage(`position fen ${fen}`);
      w.postMessage(`go movetime ${opts.movetime ?? 1000}`);
    });
  }
}

// Vanta compiled to WebAssembly (crates/wasm), run in a module Web Worker (public/vanta/worker.js).
// A running search cannot be interrupted, so every request carries a movetime; a newer request
// simply makes the older answer stale (it resolves to null).
class VantaEngine implements Engine {
  name = 'Vanta (WASM)';
  private ready: Promise<Worker> | null = null;
  private chain: Promise<unknown> = Promise.resolve();
  private seq = 0;

  private start(): Promise<Worker> {
    if (!this.ready) {
      this.ready = (async () => {
        const res = await fetch('/vanta/manifest.json');
        if (!res.ok) throw new Error('Vanta WASM is not built yet. Run chess/scripts/build-wasm.sh and commit web/public/vanta.');
        const man = (await res.json()) as { net: string };
        const w = new Worker('/vanta/worker.js', { type: 'module' });
        await new Promise<void>((resolve, reject) => {
          const done = (fn: () => void) => { w.removeEventListener('message', onMsg); w.removeEventListener('error', onErr); clearTimeout(t); fn(); };
          const onMsg = (e: MessageEvent) => {
            if (e.data?.type === 'ready') done(resolve);
            else if (e.data?.type === 'error') done(() => reject(new Error(e.data.message)));
          };
          const onErr = () => done(() => reject(new Error('The Vanta worker failed to start (is this browser too old for module workers?).')));
          const t = setTimeout(() => done(() => reject(new Error('Vanta did not start in time.'))), 30000);
          w.addEventListener('message', onMsg);
          w.addEventListener('error', onErr);
          w.postMessage({ type: 'init', net: man.net });
        });
        return w;
      })();
      this.ready.catch(() => { this.ready = null; });
    }
    return this.ready;
  }

  analyse(fen: string, opts: SearchOpts = {}): Promise<Analysis | null> {
    const mine = ++this.seq;
    const run = this.chain.then(() => this.search(fen, opts, mine));
    this.chain = run.catch(() => null);
    return run;
  }

  private async search(fen: string, opts: SearchOpts, mine: number): Promise<Analysis | null> {
    if (mine !== this.seq) return null;
    const w = await this.start();
    if (mine !== this.seq) return null;
    const sign = fen.split(' ')[1] === 'b' ? -1 : 1;
    return new Promise<Analysis | null>((resolve, reject) => {
      const onMsg = (e: MessageEvent) => {
        const d = e.data;
        if (!d || d.id !== mine) return;
        if (d.type === 'done') {
          w.removeEventListener('message', onMsg);
          const r = d.result as { best: string; score: number; mate: number; depth: number; nodes: number };
          if (!r.best || r.best === '0000') return resolve(null);
          const stm = r.mate ? (r.mate > 0 ? 10000 - r.mate : -10000 - r.mate) : r.score;
          resolve({ best: r.best, cp: sign * stm, depth: r.depth, nodes: r.nodes, pv: [r.best] });
        } else if (d.type === 'error') {
          w.removeEventListener('message', onMsg);
          reject(new Error(d.message));
        }
      };
      w.addEventListener('message', onMsg);
      w.postMessage({ type: 'go', id: mine, fen, movetime: opts.movetime ?? 1000 });
    });
  }
}

const instances: Partial<Record<EngineId, Engine>> = {};
export function getEngine(id: EngineId): Engine {
  return (instances[id] ??= id === 'stockfish' ? new StockfishEngine() : id === 'vanta' ? new VantaEngine() : new StubEngine());
}

export const engine: Engine = getEngine('stub');
