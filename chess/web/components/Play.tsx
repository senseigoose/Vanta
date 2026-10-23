'use client';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Chess } from 'chess.js';
import type { Key } from 'chessground/types';
import { ChevronLeft, ChevronRight, FileText, Play as PlayIcon, Pause, RotateCcw, SkipBack, SkipForward, Terminal, TrendingUp, Upload, Zap } from 'lucide-react';
import Board from './Board';
import { Button, Card } from './ui';
import { engine, type Analysis } from '@/lib/engine';
import { DEPTH14, VERSIONS } from '@/lib/facts';

const START = new Chess().fen();

function replay(start: string, moves: string[]) {
  const g = new Chess(start);
  for (const san of moves) g.move(san);
  return g;
}

function pvToSan(fen: string, pv: string[]) {
  const g = new Chess(fen);
  const out: string[] = [];
  for (const u of pv) {
    try { out.push(g.move({ from: u.slice(0, 2), to: u.slice(2, 4), promotion: u[4] }).san); } catch { break; }
  }
  return out;
}

const fmt = (cp: number) => `${cp >= 0 ? '+' : ''}${(cp / 100).toFixed(2)}`;

export default function Play() {
  const [start, setStart] = useState(START);
  const [moves, setMoves] = useState<string[]>([]);
  const [view, setView] = useState(0);
  const [mode, setMode] = useState<'analyse' | 'play'>('analyse');
  const [info, setInfo] = useState<Analysis | null>(null);
  const [thinking, setThinking] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  const [auto, setAuto] = useState(false);
  const [error, setError] = useState('');
  const token = useRef(0);
  const file = useRef<HTMLInputElement>(null);

  const game = useMemo(() => replay(start, moves.slice(0, view)), [start, moves, view]);
  const fen = game.fen();
  const turn = game.turn() === 'w' ? 'white' : 'black';
  const over = game.isGameOver();
  const atEnd = view === moves.length;
  const enginesTurn = mode === 'play' && !over && atEnd && turn === 'black';
  const movable = over ? undefined : mode === 'analyse' ? turn : atEnd && turn === 'white' ? turn : undefined;

  const dests = useMemo(() => {
    const d = new Map<Key, Key[]>();
    for (const m of game.moves({ verbose: true })) {
      const list = d.get(m.from as Key) ?? [];
      list.push(m.to as Key);
      d.set(m.from as Key, list);
    }
    return d;
  }, [game]);

  const last = game.history({ verbose: true }).at(-1);
  const lastMove: [string, string] | undefined = last ? [last.from, last.to] : undefined;

  const play = (from: string, to: string, promo?: string) => {
    try {
      const m = replay(start, moves.slice(0, view)).move({ from, to, promotion: promo ?? 'q' });
      setMoves([...moves.slice(0, view), m.san]);
      setView(view + 1);
    } catch { /* illegal: ignore */ }
  };

  useEffect(() => {
    const mine = ++token.current;
    setThinking(true);
    engine.analyse(fen).then((a) => {
      if (mine !== token.current) return;
      setThinking(false);
      setInfo(a);
      if (a && enginesTurn) play(a.best.slice(0, 2), a.best.slice(2, 4), a.best[4]);
    });
  }, [fen, enginesTurn]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!auto) return;
    if (atEnd) { setAuto(false); return; }
    const t = setTimeout(() => setView((v) => v + 1), 700);
    return () => clearTimeout(t);
  }, [auto, view, atEnd]);

  const reset = (s = START) => { setStart(s); setMoves([]); setView(0); setInfo(null); setDraft(null); setError(''); setAuto(false); };
  const importFen = () => {
    const f = (draft ?? fen).trim();
    try { new Chess(f); reset(f); } catch { setError('That FEN is not valid.'); }
  };
  const loadPgn = async (f?: File) => {
    if (!f) return;
    try {
      const g = new Chess();
      g.loadPgn(await f.text());
      const h = g.history();
      setStart(START); setMoves(h); setView(h.length); setMode('analyse'); setDraft(null); setError('');
    } catch { setError('Could not read that PGN.'); }
  };

  const cp = info?.cp ?? 0;
  const whitePct = 100 / (1 + Math.exp(-cp / 400));
  const pv = info ? pvToSan(fen, info.pv) : [];
  const num = fen.split(' ')[5];
  const pairs: [string, string | undefined, number][] = [];
  for (let i = 0; i < moves.length; i += 2) pairs.push([moves[i], moves[i + 1], i]);
  const cur = (n: number) => (view === n ? 'rounded-sm bg-primary/15 px-1 text-primary' : 'px-1 hover:text-primary');
  const maxElo = Math.max(...VERSIONS.map((v) => v.elo));
  const latest = VERSIONS[0];
  const tot = latest.w + latest.d + latest.l;

  return (
    <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
      <section className="min-w-0">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Play &amp; Analyze</h1>
            <p className="mt-1 text-sm text-muted-foreground">Inspect positions, compare candidates, and follow the engine’s principal variation.</p>
          </div>
          <div role="tablist" aria-label="Mode" className="flex gap-1 rounded-md border border-border bg-muted p-1 text-xs">
            {([['analyse', 'Position analysis'], ['play', 'Human vs engine']] as const).map(([k, label]) => (
              <button key={k} role="tab" aria-selected={mode === k} onClick={() => setMode(k)}
                className={`h-7 rounded px-3 ${mode === k ? 'bg-card shadow-sm' : 'text-muted-foreground'}`}>{label}</button>
            ))}
            <button role="tab" disabled title="Needs the WASM engine" className="h-7 rounded px-3 text-muted-foreground opacity-50">Engine vs engine</button>
          </div>
        </div>

        <div className="mt-6 flex flex-wrap items-center gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-input bg-card px-3">
            <Terminal className="size-4 shrink-0 text-muted-foreground" />
            <input aria-label="FEN" value={draft ?? fen} onChange={(e) => setDraft(e.target.value)}
              className="h-10 min-w-0 flex-1 bg-transparent font-mono text-xs outline-none" />
          </div>
          <Button className="h-10" onClick={importFen}><Upload className="size-4" />Import FEN</Button>
          <Button className="h-10" onClick={() => file.current?.click()}><FileText className="size-4" />Load PGN</Button>
          <input ref={file} type="file" accept=".pgn,text/plain" hidden onChange={(e) => loadPgn(e.target.files?.[0])} />
          <Button className="h-10 w-10 px-0" aria-label="Restart" onClick={() => reset()}><RotateCcw className="size-4" /></Button>
        </div>
        {error && <p role="alert" className="mt-2 text-sm text-primary">{error}</p>}

        <div className="mt-5 grid gap-5 lg:grid-cols-[minmax(0,1fr)_250px]">
          <div>
            <Card className="p-5">
              <div className="mb-4 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="flex h-8 w-2 flex-col-reverse overflow-hidden rounded-full bg-muted-foreground/40" title={`${fmt(cp)} for White`}>
                    <div className="bg-foreground" style={{ height: `${whitePct}%` }} />
                  </div>
                  <span className="text-lg font-semibold">{info ? fmt(cp) : '–'}</span>
                </div>
                <span className="rounded-md bg-secondary px-2 py-1 text-xs">Depth {info?.depth ?? '–'}</span>
              </div>
              <div className="mx-auto w-full max-w-[570px]">
                <Board fen={fen} orientation="white" turn={turn} check={game.inCheck()} lastMove={lastMove} movable={movable} dests={dests} onMove={play} />
              </div>
              <div className="mt-5 flex items-center justify-center gap-2">
                <Button variant="ghost" className="w-9 px-0" aria-label="Start" onClick={() => setView(0)} disabled={view === 0}><SkipBack className="size-4" /></Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="Back" onClick={() => setView(Math.max(0, view - 1))} disabled={view === 0}><ChevronLeft className="size-4" /></Button>
                <Button variant="solid" className="w-10 px-0" aria-label={auto ? 'Pause' : 'Play through moves'} onClick={() => setAuto(!auto)} disabled={atEnd && !auto}>
                  {auto ? <Pause className="size-4" /> : <PlayIcon className="size-4" />}
                </Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="Forward" onClick={() => setView(Math.min(moves.length, view + 1))} disabled={atEnd}><ChevronRight className="size-4" /></Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="End" onClick={() => setView(moves.length)} disabled={atEnd}><SkipForward className="size-4" /></Button>
              </div>
            </Card>
            <Card className="mt-4 p-4">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-sm font-semibold">Move history</h2>
                <span className="text-xs text-muted-foreground">Algebraic notation</span>
              </div>
              <div className="grid grid-cols-2 gap-x-8 gap-y-1 font-mono text-xs text-muted-foreground">
                {pairs.map(([w, b, i]) => (
                  <div key={i}>{i / 2 + 1}. <button className={cur(i + 1)} onClick={() => setView(i + 1)}>{w}</button>{b && <button className={cur(i + 2)} onClick={() => setView(i + 2)}>{b}</button>}</div>
                ))}
                {!moves.length && <div className="col-span-2">Make a move on the board to begin.</div>}
              </div>
              <div className="mt-4 border-t border-border pt-3 text-xs text-muted-foreground" aria-live="polite">
                Position {view} / {moves.length} · {over ? (game.isCheckmate() ? `Checkmate, ${turn === 'white' ? 'Black' : 'White'} wins` : 'Draw') : `${turn === 'white' ? 'White' : 'Black'} to move`}
              </div>
            </Card>
          </div>

          <Card className="h-fit p-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold">Engine thinking</h2>
              <span className={`size-2 rounded-full bg-primary ${thinking ? 'animate-pulse' : 'opacity-40'}`} />
            </div>
            <div className="mt-5 text-xs text-muted-foreground">Best move</div>
            <div className="mt-1 flex items-end justify-between">
              <span className="text-3xl font-semibold">{pv[0] ? `${num}.${turn === 'black' ? '..' : ''} ${pv[0]}` : '–'}</span>
              <span className="text-lg font-medium text-primary">{info ? fmt(cp) : ''}</span>
            </div>
            <div className="mt-2 flex justify-between text-xs text-muted-foreground"><span>Nodes</span><span className="text-foreground">{info?.nodes ?? '–'}</span></div>
            <Button variant="solid" className="mt-4 w-full" disabled={!info || over}
              onClick={() => info && play(info.best.slice(0, 2), info.best.slice(2, 4), info.best[4])}><Zap className="size-4" />Play best move</Button>
            <div className="mt-6 border-t border-border pt-4">
              <div className="text-xs font-medium">Principal variation</div>
              <p className="mt-2 font-mono text-[11px] leading-5 text-muted-foreground">{pv.join(' ') || '–'}</p>
            </div>
            <p className="mt-4 text-[11px] text-muted-foreground">{engine.name}. Candidate moves need MultiPV, which is not built yet.</p>
          </Card>
        </div>
      </section>

      <aside className="flex flex-col gap-5">
        <Card className="p-5">
          <h2 className="font-semibold">Current net</h2>
          <div className="mt-4 grid grid-cols-2 gap-2">
            {[['Training positions', '128M'], ['Parameters', '197,377'], ['Epochs', '40'], ['Batch size', '16,384']].map(([k, v]) => (
              <div key={k} className="rounded-md bg-muted p-3"><div className="text-[11px] text-muted-foreground">{k}</div><div className="mt-1 text-lg font-semibold">{v}</div></div>
            ))}
          </div>
          <p className="mt-4 text-xs text-muted-foreground">768 → 256 ×2 → 1, trained offline on Kaggle.</p>
        </Card>
        <Card className="p-5">
          <div className="flex items-center justify-between"><h2 className="font-semibold">Strength snapshot</h2><TrendingUp className="size-4 text-chart-2" /></div>
          <div className="mt-4 text-4xl font-semibold">≈2,400</div>
          <div className="mt-1 text-xs text-muted-foreground">Rating vs limited Stockfish (v0.11, rough, ±100)</div>
          <div className="mt-3 text-sm text-chart-2">+{latest.elo.toFixed(0)} Elo from {latest.tag}</div>
          <div className="mt-3 grid grid-cols-3 text-center text-xs">
            {[['Win', latest.w], ['Draw', latest.d], ['Loss', latest.l]].map(([k, n]) => (
              <div key={k}><div className="font-semibold">{Math.round((Number(n) / tot) * 100)}%</div><div className="text-muted-foreground">{k}</div></div>
            ))}
          </div>
        </Card>
        <Card className="p-5">
          <h2 className="font-semibold">Engine health</h2>
          <div className="mt-4 space-y-3 text-xs">
            <div><div className="mb-1 flex justify-between"><span>Search speed (native)</span><span>3.3M nodes/s</span></div><div className="h-1 rounded-full bg-muted"><div className="h-full w-[80%] rounded-full bg-primary" /></div></div>
            <div className="flex justify-between"><span>Move generator</span><span className="flex items-center gap-2"><span className="size-2 rounded-full bg-chart-2" />perft verified</span></div>
            <div className="flex justify-between"><span>Browser engine</span><span className="flex items-center gap-2"><span className="size-2 rounded-full bg-chart-3" />stub for now</span></div>
          </div>
        </Card>
      </aside>

      <section className="min-w-0 xl:col-span-2">
        <div className="grid gap-5 md:grid-cols-2">
          <Card className="p-4">
            <div className="flex items-center justify-between"><h2 className="text-sm font-semibold">Elo gain per version</h2><span className="text-xs text-muted-foreground">at 2+0.02</span></div>
            <div className="mt-3 space-y-2 text-xs">
              {VERSIONS.map((v) => (
                <div key={v.tag} className="flex items-center gap-3"><span className="w-10 text-muted-foreground">{v.tag}</span>
                  <div className="h-2 flex-1 rounded-full bg-muted"><div className="h-full rounded-full bg-chart-1" style={{ width: `${(v.elo / maxElo) * 100}%` }} /></div>
                  <span className="w-12 text-right">+{v.elo.toFixed(0)}</span></div>
              ))}
            </div>
          </Card>
          <Card className="p-4">
            <div className="flex items-center justify-between"><h2 className="text-sm font-semibold">Nodes to depth 14</h2><span className="text-xs text-muted-foreground">millions, start position</span></div>
            <div className="mt-3 space-y-2 text-xs">
              {DEPTH14.map((d) => (
                <div key={d.label} className="flex items-center gap-3"><span className="w-24 text-muted-foreground">{d.label}</span>
                  <div className="h-2 flex-1 rounded-full bg-muted"><div className="h-full rounded-full bg-chart-3" style={{ width: `${(d.nodes / 4.47) * 100}%` }} /></div>
                  <span className="w-10 text-right">{d.nodes}</span></div>
              ))}
            </div>
          </Card>
        </div>
        <Card className="mt-5 overflow-hidden">
          <h2 className="border-b border-border px-5 py-4 text-sm font-semibold">Recent versions</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="bg-muted text-muted-foreground"><tr>{['Version', 'Change', 'Elo vs previous', 'Win / draw / loss', 'Status'].map((h) => <th key={h} className="px-5 py-3 font-medium">{h}</th>)}</tr></thead>
              <tbody>
                {VERSIONS.map((v) => (
                  <tr key={v.tag} className="border-t border-border">
                    <td className="px-5 py-3 font-medium">{v.tag}</td><td className="px-5 py-3 text-muted-foreground">{v.change}</td>
                    <td className="px-5 py-3">+{v.elo.toFixed(1)} ± {v.err.toFixed(1)}</td><td className="px-5 py-3">{v.w} / {v.d} / {v.l}</td>
                    <td className="px-5 py-3">{v.status === 'Current' ? <span className="rounded bg-chart-2/15 px-2 py-0.5 text-chart-2">Current</span> : <span className="text-muted-foreground">Archived</span>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      </section>
    </div>
  );
}
