'use client';
import { useEffect, useMemo, useState } from 'react';
import { Card, Head, Table } from '@/components/ui';
import { H, NIN, PARAMS, evaluate, parseNet, type Net } from '@/lib/nnue';

const IDS = ['v1', 'v2', 'v3', 'v4'];
const PIECES = ['Pawn', 'Knight', 'Bishop', 'Rook', 'Queen', 'King'];
const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

export default function Page() {
  const [nets, setNets] = useState<Record<string, Net>>({});
  const [missing, setMissing] = useState(false);
  const [which, setWhich] = useState('v4');
  const [neuron, setNeuron] = useState(0);
  const [fen, setFen] = useState(START);

  useEffect(() => {
    IDS.forEach(async (id) => {
      try {
        const r = await fetch(`/nets/vanta-${id}.nnue`);
        if (!r.ok) throw new Error();
        const net = parseNet(await r.arrayBuffer());
        setNets((n) => ({ ...n, [id]: net }));
      } catch { setMissing(true); }
    });
  }, []);

  const net = nets[which];
  const maxAbs = useMemo(() => {
    if (!net) return 1;
    let m = 1;
    for (let f = 0; f < NIN; f++) m = Math.max(m, Math.abs(net.fw[f * H + neuron]));
    return m;
  }, [net, neuron]);
  const scores = IDS.filter((id) => nets[id]).map((id) => { try { return [`vanta-${id}`, evaluate(nets[id], fen)] as const; } catch { return [`vanta-${id}`, NaN] as const; } });

  return (
    <div className="max-w-5xl space-y-5">
      <Head title="Model Inspector" sub="What the network has learned. Nets are read in your browser from public/nets." />
      {missing && <p role="alert" className="rounded-md border border-border bg-card p-3 text-sm">Some nets could not be loaded. Copy the .nnue files into chess/web/public/nets/ (see the notes that came with this build).</p>}
      <Card className="p-5">
        <div className="flex flex-wrap items-center gap-4 text-sm">
          <label>Net <select value={which} onChange={(e) => setWhich(e.target.value)} className="ml-2 h-9 rounded-md border border-border bg-background px-2">{IDS.map((i) => <option key={i} value={i}>vanta-{i}</option>)}</select></label>
          <span className="text-muted-foreground">768 → 256 ×2 → 1 · {PARAMS.toLocaleString()} parameters</span>
        </div>
        <label className="mt-4 block text-sm">Hidden neuron {neuron} of {H - 1}
          <input type="range" min={0} max={H - 1} value={neuron} onChange={(e) => setNeuron(Number(e.target.value))} className="mt-2 block w-full accent-[var(--color-primary)]" />
        </label>
        <p className="mt-2 text-xs text-muted-foreground">Each board shows how strongly one piece on each square pushes this neuron up (red) or down (blue). First row: your pieces. Second row: the opponent’s.</p>
        {net ? (
          <div className="mt-4 grid grid-cols-3 gap-3 sm:grid-cols-6">
            {[0, 1].flatMap((side) => PIECES.map((name, pc) => (
              <figure key={`${side}${pc}`}>
                <div className="grid aspect-square grid-cols-8 overflow-hidden rounded border border-border">
                  {Array.from({ length: 64 }, (_, k) => {
                    const sq = (7 - Math.floor(k / 8)) * 8 + (k % 8);
                    const w = net.fw[(side * 384 + pc * 64 + sq) * H + neuron];
                    const a = Math.min(1, Math.abs(w) / maxAbs);
                    return <div key={k} style={{ background: w >= 0 ? `rgba(239,68,68,${a})` : `rgba(59,130,246,${a})` }} />;
                  })}
                </div>
                <figcaption className="mt-1 text-center text-[11px] text-muted-foreground">{side ? 'Their' : 'Your'} {name.toLowerCase()}</figcaption>
              </figure>
            )))}
          </div>
        ) : <p className="mt-4 text-sm text-muted-foreground">Loading…</p>}
      </Card>
      <Card className="overflow-hidden">
        <div className="p-5">
          <h2 className="text-sm font-semibold">Same position, every net</h2>
          <input aria-label="FEN" value={fen} onChange={(e) => setFen(e.target.value)} className="mt-3 h-10 w-full rounded-md border border-input bg-background px-3 font-mono text-xs outline-none focus-visible:outline-2 focus-visible:outline-primary" />
          <p className="mt-2 text-xs text-muted-foreground">Static evaluation in centipawns for the side to move, with no search.</p>
        </div>
        <Table heads={['Net', 'Evaluation']} rows={scores.map(([n, s]) => [n, Number.isNaN(s) ? 'invalid FEN' : (s >= 0 ? '+' : '') + s])} />
      </Card>
    </div>
  );
}
