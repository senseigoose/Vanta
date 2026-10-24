'use client';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Chess } from 'chess.js';
import type { Key } from 'chessground/types';
import { Download, Pause, Play as PlayIcon, RotateCcw } from 'lucide-react';
import Board from './Board';
import { Button, Card } from './ui';
import { ENGINES, getEngine, type Analysis, type EngineId } from '@/lib/engine';

type Side = { id: EngineId; limited: boolean; elo: number };

const PLAYERS = ENGINES.filter((e) => e.id !== 'stub');
const MAX_PLIES = 300; // call it a draw past this, so a match can never run forever
const NO_DESTS = new Map<Key, Key[]>();
const fmt = (cp: number) => `${cp >= 0 ? '+' : ''}${(cp / 100).toFixed(2)}`;

function label(s: Side) {
  const name = PLAYERS.find((e) => e.id === s.id)?.label ?? s.id;
  return s.id === 'stockfish' && s.limited ? `${name} (about ${s.elo})` : name;
}

function replay(moves: string[]) {
  const g = new Chess();
  for (const san of moves) g.move(san);
  return g;
}

function outcome(g: Chess, plies: number): { result: string; why: string } {
  if (g.isCheckmate()) return { result: g.turn() === 'w' ? '0-1' : '1-0', why: 'checkmate' };
  if (g.isStalemate()) return { result: '1/2-1/2', why: 'stalemate' };
  if (g.isThreefoldRepetition()) return { result: '1/2-1/2', why: 'threefold repetition' };
  if (g.isInsufficientMaterial()) return { result: '1/2-1/2', why: 'insufficient material' };
  if (g.isDraw()) return { result: '1/2-1/2', why: 'fifty-move rule' };
  if (plies >= MAX_PLIES) return { result: '1/2-1/2', why: `ply cap (${MAX_PLIES})` };
  return { result: '*', why: '' };
}

function SidePicker({ title, side, onChange, disabled }: { title: string; side: Side; onChange: (s: Side) => void; disabled: boolean }) {
  return (
    <div className="space-y-2 rounded-lg border border-border p-3 text-xs">
      <div className="text-sm font-semibold">{title}</div>
      <label className="flex items-center justify-between gap-2">Engine
        <select value={side.id} disabled={disabled} onChange={(e) => onChange({ ...side, id: e.target.value as EngineId })}
          className="h-7 rounded border border-input bg-background px-2">
          {PLAYERS.map((x) => <option key={x.id} value={x.id}>{x.label}</option>)}
        </select>
      </label>
      <label className="flex items-center gap-2">
        <input type="checkbox" checked={side.limited} disabled={disabled || side.id !== 'stockfish'} onChange={(e) => onChange({ ...side, limited: e.target.checked })} />
        Limit strength (Stockfish only)
      </label>
      <label className="flex items-center gap-2">Elo
        <input type="range" min={1320} max={3190} step={10} value={side.elo} disabled={disabled || side.id !== 'stockfish' || !side.limited}
          onChange={(e) => onChange({ ...side, elo: Number(e.target.value) })} />
        <span className="w-10 tabular-nums">{side.elo}</span>
      </label>
    </div>
  );
}

export default function Match() {
  const [white, setWhite] = useState<Side>({ id: 'vanta', limited: false, elo: 2000 });
  const [black, setBlack] = useState<Side>({ id: 'stockfish', limited: true, elo: 2000 });
  const [movetime, setMovetime] = useState(500);
  const [moves, setMoves] = useState<string[]>([]);
  const [running, setRunning] = useState(false);
  const [thinking, setThinking] = useState(false);
  const [info, setInfo] = useState<Analysis | null>(null);
  const [error, setError] = useState('');
  const token = useRef(0);

  const game = useMemo(() => replay(moves), [moves]);
  const fen = game.fen();
  const turn = game.turn() === 'w' ? 'white' : 'black';
  const end = outcome(game, moves.length);
  const over = end.result !== '*';
  const last = game.history({ verbose: true }).at(-1);
  const lastMove: [string, string] | undefined = last ? [last.from, last.to] : undefined;

  useEffect(() => {
    if (!running || over) return;
    const mine = ++token.current;
    const side = game.turn() === 'w' ? white : black;
    setThinking(true);
    getEngine(side.id)
      .analyse(fen, { movetime, elo: side.id === 'stockfish' && side.limited ? side.elo : undefined })
      .then((a) => {
        if (mine !== token.current) return;
        setThinking(false);
        setError('');
        if (!a) { setRunning(false); setError('The engine returned no move.'); return; }
        setInfo(a);
        try {
          const g = new Chess(fen);
          const m = g.move({ from: a.best.slice(0, 2), to: a.best.slice(2, 4), promotion: a.best[4] });
          setMoves((prev) => [...prev, m.san]);
        } catch {
          setRunning(false);
          setError(`The engine played an illegal move: ${a.best}`);
        }
      })
      .catch((e) => {
        if (mine !== token.current) return;
        setThinking(false);
        setRunning(false);
        setError(e instanceof Error ? e.message : 'The engine failed.');
      });
    return () => { token.current++; };
  }, [running, fen, over]); // eslint-disable-line react-hooks/exhaustive-deps

  const reset = () => { setRunning(false); setMoves([]); setInfo(null); setError(''); setThinking(false); };

  const download = () => {
    const g = replay(moves);
    g.setHeader('Event', 'Vanta site match');
    g.setHeader('White', label(white));
    g.setHeader('Black', label(black));
    g.setHeader('Result', end.result);
    const blob = new Blob([g.pgn()], { type: 'application/x-chess-pgn' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'vanta-match.pgn';
    a.click();
    URL.revokeObjectURL(url);
  };

  const pairs: [string, string | undefined, number][] = [];
  for (let i = 0; i < moves.length; i += 2) pairs.push([moves[i], moves[i + 1], i]);
  const started = moves.length > 0;

  return (
    <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section className="min-w-0">
        <h1 className="text-2xl font-semibold tracking-tight">Engine Match</h1>
        <p className="mt-1 text-sm text-muted-foreground">Pick two engines and watch them play. Everything runs in your browser; a finished game downloads as PGN and opens in Game Explorer.</p>
        <Card className="mt-5 p-4">
          <div className="mx-auto w-full max-w-[570px]">
            <Board fen={fen} orientation="white" turn={turn} check={game.inCheck()} lastMove={lastMove} movable={undefined} dests={NO_DESTS} onMove={() => {}} />
          </div>
          <div className="mt-4 flex flex-wrap items-center gap-3">
            <Button variant="solid" onClick={() => setRunning((r) => !r)} disabled={over}>
              {running ? <><Pause className="size-4" />Pause</> : <><PlayIcon className="size-4" />{started ? 'Resume' : 'Start'}</>}
            </Button>
            <Button onClick={reset}><RotateCcw className="size-4" />New game</Button>
            <Button onClick={download} disabled={!started}><Download className="size-4" />Download PGN</Button>
            <span className="text-sm text-muted-foreground" aria-live="polite">
              {over ? `${end.result} by ${end.why}` : running ? (thinking ? `${turn === 'white' ? label(white) : label(black)} is thinking…` : 'Playing…') : started ? 'Paused' : 'Ready'}
            </span>
          </div>
          {error && <p role="alert" className="mt-3 text-sm text-primary">{error}</p>}
        </Card>
      </section>
      <aside className="space-y-4">
        <Card className="space-y-3 p-4">
          <SidePicker title="White" side={white} onChange={setWhite} disabled={running} />
          <SidePicker title="Black" side={black} onChange={setBlack} disabled={running} />
          <label className="flex items-center justify-between gap-2 text-xs">Time per move
            <select value={movetime} disabled={running} onChange={(e) => setMovetime(Number(e.target.value))} className="h-7 rounded border border-input bg-background px-2">
              {[200, 500, 1000, 2000].map((t) => <option key={t} value={t}>{t} ms</option>)}
            </select>
          </label>
        </Card>
        <Card className="p-4">
          <h2 className="text-sm font-semibold">Last move</h2>
          <div className="mt-2 text-xs text-muted-foreground">
            {info ? <>Evaluation {fmt(info.cp)} for White · depth {info.depth} · {info.nodes.toLocaleString('en-US')} nodes</> : 'No move yet.'}
          </div>
        </Card>
        <Card className="max-h-80 overflow-y-auto p-4">
          <h2 className="text-sm font-semibold">Moves</h2>
          <div className="mt-2 grid grid-cols-[2rem_1fr_1fr] gap-x-2 gap-y-1 font-mono text-xs">
            {pairs.length === 0 && <span className="col-span-3 text-muted-foreground">Press Start.</span>}
            {pairs.map(([w, b, i]) => (
              <div key={i} className="contents"><span className="text-muted-foreground">{i / 2 + 1}.</span><span>{w}</span><span>{b ?? ''}</span></div>
            ))}
          </div>
        </Card>
      </aside>
    </div>
  );
}
