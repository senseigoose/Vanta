import { Card, Head, Table } from '@/components/ui';
import { EXPERIMENTS } from '@/lib/facts';

export default function Page() {
  return (
    <div className="max-w-5xl space-y-5">
      <Head title="Experiments & Logs" sub="Every test result, including the ones that did not pass. Newest first." />
      <Card className="overflow-hidden">
        <Table heads={['Experiment', 'Compared with', 'Elo', 'Games', 'Outcome', 'Commit']}
          rows={EXPERIMENTS.map((e) => [e.what, e.vs, `${e.elo >= 0 ? '+' : ''}${e.elo.toFixed(1)} ± ${e.err.toFixed(1)}`, e.games, e.outcome, e.ref])} />
      </Card>
      <p className="text-xs text-muted-foreground">All at 2+0.02 unless stated. “Inconclusive” means the run reached its round cap without crossing a bound. Failed games and timeouts are not recorded yet.</p>
    </div>
  );
}
