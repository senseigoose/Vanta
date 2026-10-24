#!/usr/bin/env python3
"""Hook Stockfish (WASM, in a Web Worker) into the Vanta site.
Run from /workspaces/Vanta/chess/web:   python3 add_stockfish.py
Writes lib/engine.ts and scripts/get-stockfish.sh, and patches components/Play.tsx."""
import os, stat

ENGINE_TS = r'''import { Chess } from 'chess.js';

// White's point of view, in centipawns. `best` and `pv` are UCI moves like "e2e4" or "e7e8q".
export type Analysis = { best: string; cp: number; depth: number; nodes: number; pv: string[] };

// `elo` (Stockfish only): play at about this strength. Leave it out for full strength.
export type SearchOpts = { movetime?: number; elo?: number };

export interface Engine {
  name: string;
  analyse(fen: string, opts?: SearchOpts): Promise<Analysis | null>;
}

export type EngineId = 'stockfish' | 'stub';
export const ENGINES: { id: EngineId; label: string }[] = [
  { id: 'stockfish', label: 'Stockfish (WASM)' },
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

const instances: Partial<Record<EngineId, Engine>> = {};
export function getEngine(id: EngineId): Engine {
  return (instances[id] ??= id === 'stockfish' ? new StockfishEngine() : new StubEngine());
}

export const engine: Engine = getEngine('stub');
'''

GET_SH = r'''#!/usr/bin/env bash
# Download the Stockfish.js lite single-threaded build (GPLv3) into public/stockfish.
# Run from chess/web:  bash scripts/get-stockfish.sh   then commit public/stockfish.
set -euo pipefail
VER="v19.0.1"
NAME="stockfish-19-lite-single"
BASE="https://github.com/nmrugg/stockfish.js/releases/download/$VER"
DIR="public/stockfish"
mkdir -p "$DIR"
curl -fL -o "$DIR/$NAME.js" "$BASE/$NAME.js"
curl -fL -o "$DIR/$NAME.wasm" "$BASE/$NAME.wasm"
cat > "$DIR/manifest.json" << EOF
{ "js": "$NAME.js", "name": "Stockfish 19 lite (WASM, single thread)" }
EOF
cat > "$DIR/NOTICE.txt" << EOF
Stockfish is free software under the GNU General Public License v3.
This is Stockfish.js by Nathan Rugg, built from Stockfish: https://github.com/nmrugg/stockfish.js
Stockfish source: https://github.com/official-stockfish/Stockfish
EOF
ls -l "$DIR"
'''

def patch(path, pairs):
    s = open(path, encoding="utf-8").read()
    for old, new in pairs:
        assert s.count(old) == 1, f"anchor not found exactly once: {old[:60]!r}"
        s = s.replace(old, new)
    open(path, "w", encoding="utf-8").write(s)

assert os.path.isfile("components/Play.tsx"), "run this from chess/web"
os.makedirs("scripts", exist_ok=True)
open("lib/engine.ts", "w", encoding="utf-8").write(ENGINE_TS)
open("scripts/get-stockfish.sh", "w", encoding="utf-8", newline="\n").write(GET_SH)
os.chmod("scripts/get-stockfish.sh", os.stat("scripts/get-stockfish.sh").st_mode | stat.S_IXUSR)

SELECTOR = r'''        <div className="mt-4 flex flex-wrap items-center gap-4 rounded-md border border-border bg-card px-3 py-2 text-xs">
          <label className="flex items-center gap-2">Engine
            <select value={engineId} onChange={(e) => setEngineId(e.target.value as EngineId)} className="h-7 rounded border border-input bg-background px-2">
              {ENGINES.map((x) => <option key={x.id} value={x.id}>{x.label}</option>)}
            </select>
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={limited} disabled={engineId !== 'stockfish'} onChange={(e) => setLimited(e.target.checked)} />
            Limit strength (Human vs engine)
          </label>
          <label className="flex items-center gap-2">Elo
            <input type="range" min={1320} max={3190} step={10} value={elo} disabled={engineId !== 'stockfish' || !limited} onChange={(e) => setElo(Number(e.target.value))} />
            <span className="w-10 tabular-nums">{elo}</span>
          </label>
        </div>

'''

patch("components/Play.tsx", [
    ("import { engine, type Analysis } from '@/lib/engine';",
     "import { ENGINES, getEngine, type Analysis, type EngineId } from '@/lib/engine';"),
    ("  const [error, setError] = useState('');\n",
     "  const [error, setError] = useState('');\n"
     "  const [engineId, setEngineId] = useState<EngineId>('stockfish');\n"
     "  const [limited, setLimited] = useState(true);\n"
     "  const [elo, setElo] = useState(1800);\n"),
    ("""    engine.analyse(fen).then((a) => {
      if (mine !== token.current) return;
      setThinking(false);
      setInfo(a);
      if (a && enginesTurn) play(a.best.slice(0, 2), a.best.slice(2, 4), a.best[4]);
    });
  }, [fen, enginesTurn]); // eslint-disable-line react-hooks/exhaustive-deps""",
     """    const play_ = mode === 'play';
    getEngine(engineId).analyse(fen, {
      movetime: play_ ? 1000 : 1500,
      elo: play_ && limited && engineId === 'stockfish' ? elo : undefined,
    }).then((a) => {
      if (mine !== token.current) return;
      setThinking(false);
      setInfo(a);
      setError('');
      if (a && enginesTurn) play(a.best.slice(0, 2), a.best.slice(2, 4), a.best[4]);
    }).catch((e) => {
      if (mine !== token.current) return;
      setThinking(false);
      setError(e instanceof Error ? e.message : 'The engine failed.');
    });
  }, [fen, enginesTurn, engineId]); // eslint-disable-line react-hooks/exhaustive-deps"""),
    ('        <div className="mt-6 flex flex-wrap items-center gap-3">',
     SELECTOR + '        <div className="mt-6 flex flex-wrap items-center gap-3">'),
    ("{engine.name}. Candidate moves", "{getEngine(engineId).name}. Candidate moves"),
    ("stub for now</span>", "{engineId === 'stockfish' ? 'Stockfish (Vanta WASM not built)' : 'stub'}</span>"),
])
print("ok: wrote lib/engine.ts, scripts/get-stockfish.sh; patched components/Play.tsx")
