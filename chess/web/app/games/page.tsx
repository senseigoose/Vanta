'use client';
import { useMemo, useRef, useState } from 'react';
import { Chess } from 'chess.js';
import { ChevronLeft, ChevronRight, Download, FileText, SkipBack, SkipForward } from 'lucide-react';
import Board from '@/components/Board';
import { Button, Card, Head } from '@/components/ui';

type G = { pgn: string; white: string; black: string; result: string; plies: number };
const NO = new Map();

export default function Page() {
  const [games, setGames] = useState<G[]>([]);
  const [sel, setSel] = useState<number | null>(null);
  const [ply, setPly] = useState(0);
  const [filter, setFilter] = useState('all');
  const [err, setErr] = useState('');
  const file = useRef<HTMLInputElement>(null);

  const load = async (f?: File) => {
    if (!f) return;
    const out: G[] = [];
    for (const chunk of (await f.text()).split(/\r?\n\r?\n(?=\[Event )/)) {
      try {
        const g = new Chess();
        g.loadPgn(chunk);
        const h = g.getHeaders();
        out.push({ pgn: chunk, white: h.White ?? '?', black: h.Black ?? '?', result: h.Result ?? '*', plies: g.history().length });
      } catch { /* skip unreadable game */ }
    }
    setErr(out.length ? '' : 'No games found in that file.');
    setGames(out); setSel(null); setPly(0);
  };

  const shown = useMemo(() => games.map((g, i) => [g, i] as const).filter(([g]) => filter === 'all' || g.result === filter).slice(0, 300), [games, filter]);
  const moves = useMemo(() => {
    if (sel === null) return [];
    const g = new Chess();
    g.loadPgn(games[sel].pgn);
    return g.history();
  }, [games, sel]);
  const pos = useMemo(() => { const g = new Chess(); for (const m of moves.slice(0, ply)) g.move(m); return g; }, [moves, ply]);
  const last = pos.history({ verbose: true }).at(-1);
  const download = () => {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([games[sel!].pgn], { type: 'text/plain' }));
    a.download = 'game.pgn'; a.click();
  };
  const open = (i: number) => { setSel(i); setPly(0); };

  return (
    <div className="max-w-6xl">
      <Head title="Game Explorer" sub="Replay games from a PGN file, such as the games.pgn that the SPRT workflow saves." />
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <Button onClick={() => file.current?.click()}><FileText className="size-4" />Load PGN file</Button>
        <input ref={file} type="file" accept=".pgn,text/plain" hidden onChange={(e) => load(e.target.files?.[0])} />
        <select aria-label="Result" value={filter} onChange={(e) => setFilter(e.target.value)} className="h-9 rounded-md border border-border bg-background px-2 text-sm">
          <option value="all">All results</option><option value="1-0">White won</option><option value="0-1">Black won</option><option value="1/2-1/2">Draws</option>
        </select>
        <span className="text-xs text-muted-foreground">{games.length ? `${games.length} games loaded` : 'No file loaded'}</span>
        {err && <span role="alert" className="text-sm text-primary">{err}</span>}
      </div>
      <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_320px]">
        <Card className="p-5">
          {sel === null ? <p className="text-sm text-muted-foreground">Pick a game from the list to replay it.</p> : (
            <>
              <div className="mx-auto w-full max-w-[520px]">
                <Board fen={pos.fen()} orientation="white" turn={pos.turn() === 'w' ? 'white' : 'black'} check={pos.inCheck()}
                  lastMove={last ? [last.from, last.to] : undefined} movable={undefined} dests={NO} onMove={() => {}} />
              </div>
              <div className="mt-4 flex items-center justify-center gap-2">
                <Button variant="ghost" className="w-9 px-0" aria-label="Start" onClick={() => setPly(0)} disabled={ply === 0}><SkipBack className="size-4" /></Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="Back" onClick={() => setPly(Math.max(0, ply - 1))} disabled={ply === 0}><ChevronLeft className="size-4" /></Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="Forward" onClick={() => setPly(Math.min(moves.length, ply + 1))} disabled={ply === moves.length}><ChevronRight className="size-4" /></Button>
                <Button variant="ghost" className="w-9 px-0" aria-label="End" onClick={() => setPly(moves.length)} disabled={ply === moves.length}><SkipForward className="size-4" /></Button>
                <Button onClick={download}><Download className="size-4" />Export PGN</Button>
              </div>
              <p className="mt-3 text-center text-xs text-muted-foreground">{games[sel].white} vs {games[sel].black} · {games[sel].result} · move {ply} of {moves.length}</p>
            </>
          )}
        </Card>
        <Card className="max-h-[560px] overflow-auto p-2">
          {shown.map(([g, i]) => (
            <button key={i} onClick={() => open(i)} className={`block w-full rounded px-3 py-2 text-left text-xs hover:bg-muted ${sel === i ? 'bg-primary/15 text-primary' : ''}`}>
              {i + 1}. {g.white} vs {g.black} <span className="text-muted-foreground">· {g.result} · {g.plies} plies</span>
            </button>
          ))}
          {!shown.length && <p className="p-3 text-xs text-muted-foreground">Nothing to show.</p>}
        </Card>
      </div>
      <p className="mt-4 text-xs text-muted-foreground">Evaluation swings and missed-tactic marks need the engine in the browser. Not built yet.</p>
    </div>
  );
}
