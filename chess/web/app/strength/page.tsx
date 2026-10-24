import { Card, Head, Table } from '@/components/ui';
import LineChart from '@/components/LineChart';
import { ANCHOR, HISTORY, VERSIONS } from '@/lib/facts';

export default function Page() {
  const steps = [...HISTORY, ...[...VERSIONS].reverse()];
  let sum = 0;
  const ys = [0, ...steps.map((s) => (sum += s.elo))];
  return (
    <div className="max-w-5xl space-y-5">
      <Head title="Strength & Evaluation" sub="Whether each change made the engine stronger." />
      <Card className="p-5">
        <h2 className="text-sm font-semibold">Elo gained across versions</h2>
        <p className="mb-3 mt-1 text-xs text-muted-foreground">Each step is measured against the version before it, at 2+0.02. Adding them up overstates real strength; use it to see the shape.</p>
        <LineChart series={[{ name: 'Sum of self-play gains', color: 'var(--color-chart-1)', ys }]} fmt={(n) => `+${Math.round(n)}`} />
      </Card>
      <Card className="overflow-hidden">
        <h2 className="border-b border-border px-5 py-4 text-sm font-semibold">Recent versions</h2>
        <Table heads={['Version', 'Change', 'Elo vs previous', 'Win / draw / loss']} rows={VERSIONS.map((v) => [v.tag, v.change, `+${v.elo.toFixed(1)} ± ${v.err.toFixed(1)}`, `${v.w} / ${v.d} / ${v.l}`])} />
      </Card>
      <Card className="overflow-hidden">
        <h2 className="border-b border-border px-5 pt-4 text-sm font-semibold">Rating anchor: v0.11 against limited Stockfish</h2>
        <p className="px-5 pb-4 pt-1 text-xs text-muted-foreground">10+0.1, 100 games per level. Roughly 2,400 on Stockfish’s own scale, give or take 100. Levels where Vanta scored under 15% are unreliable.</p>
        <Table heads={['Stockfish level', 'Vanta score', 'Implied rating']} rows={ANCHOR.map((a) => [a.level, `${a.score}%`, `${a.rating} ± ${a.err}`])} />
      </Card>
      <p className="text-xs text-muted-foreground">Results by colour, opening and game phase, and puzzle accuracy, need the game files and a puzzle set. Not built yet.</p>
    </div>
  );
}
