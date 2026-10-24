import { Card, Head, Table } from '@/components/ui';
import LineChart from '@/components/LineChart';
import { GAME_ENDS, LOSS_V1, LOSS_V2, NETS } from '@/lib/facts';

export default function Page() {
  return (
    <div className="max-w-5xl space-y-5">
      <Head title="Training Lab" sub="How each net was made. Vanta trains offline, so this is a record of runs, not a live monitor." />
      <Card className="p-5">
        <h2 className="text-sm font-semibold">Training loss</h2>
        <p className="mb-3 mt-1 text-xs text-muted-foreground">One point per pass over the data. The two runs use different data, so their levels are not directly comparable: the 8M run partly memorised its small set. Logs for v3 and v4 were not kept.</p>
        <LineChart fmt={(n) => n.toFixed(4)} series={[
          { name: 'vanta-v1 (8M positions)', color: 'var(--color-chart-1)', ys: LOSS_V1, xs: LOSS_V1.map((_, i) => i + 1) },
          { name: 'vanta-v2 (48M positions, sampled)', color: 'var(--color-chart-3)', ys: LOSS_V2.ys, xs: LOSS_V2.xs },
        ]} />
      </Card>
      <Card className="overflow-hidden">
        <h2 className="border-b border-border px-5 py-4 text-sm font-semibold">Nets</h2>
        <Table heads={['Net', 'Positions', 'Data shards', 'Final loss', 'Result']} rows={NETS.map((n) => [n.id, n.positions, n.shards, n.loss, n.note])} />
      </Card>
      <Card className="p-5">
        <h2 className="text-sm font-semibold">Settings (all nets)</h2>
        <dl className="mt-3 grid grid-cols-2 gap-x-8 gap-y-2 text-xs sm:grid-cols-3">
          {[['Architecture', '768 → 256 ×2 → 1, SCReLU'], ['Batch size', '16,384'], ['Epochs', '40'], ['Learning rate', '0.001, ×0.1 every 15 epochs'], ['Optimiser', 'AdamW'], ['Score / result blend', '70% / 30%'], ['Eval scale', '400'], ['Data', 'self-play, 5,000 nodes per move, handcrafted eval'], ['Seeds', 'one per data shard (shard id)']].map(([k, v]) => (
            <div key={k}><dt className="text-muted-foreground">{k}</dt><dd className="mt-0.5">{v}</dd></div>
          ))}
        </dl>
      </Card>
      <Card className="overflow-hidden">
        <h2 className="border-b border-border px-5 pt-4 text-sm font-semibold">How self-play games ended</h2>
        <p className="px-5 pb-4 pt-1 text-xs text-muted-foreground">Two 30,000-position runs, same seed. NNUE self-play was not used for training: its scores ran about 1.5× the search score the net was trained on.</p>
        <Table heads={['End of game', 'Handcrafted eval', 'NNUE']} rows={GAME_ENDS.map(([k, a, b]) => [k, `${a}%`, `${b}%`])} />
      </Card>
    </div>
  );
}
