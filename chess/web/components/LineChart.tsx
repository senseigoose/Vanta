type Series = { name: string; color: string; ys: number[]; xs?: number[] };

export default function LineChart({ series, fmt = String }: { series: Series[]; fmt?: (n: number) => string }) {
  const W = 600, Hh = 180, padL = 56, padB = 22;
  const ys = series.flatMap((s) => s.ys);
  const lo = Math.min(...ys), hi = Math.max(...ys);
  const xmax = Math.max(...series.map((s) => (s.xs ? Math.max(...s.xs) : s.ys.length - 1)));
  const xmin = Math.min(...series.map((s) => (s.xs ? Math.min(...s.xs) : 0)));
  const X = (x: number) => padL + ((x - xmin) / (xmax - xmin || 1)) * (W - padL - 8);
  const Y = (y: number) => 8 + (1 - (y - lo) / (hi - lo || 1)) * (Hh - padB - 8);
  return (
    <figure>
      <svg viewBox={`0 0 ${W} ${Hh}`} className="w-full" role="img" aria-label={series.map((s) => s.name).join(', ')}>
        <line x1={padL} x2={W - 8} y1={Y(lo)} y2={Y(lo)} stroke="var(--color-border)" />
        <text x={padL - 6} y={Y(hi) + 4} textAnchor="end" fontSize="11" fill="var(--color-muted-foreground)">{fmt(hi)}</text>
        <text x={padL - 6} y={Y(lo) + 4} textAnchor="end" fontSize="11" fill="var(--color-muted-foreground)">{fmt(lo)}</text>
        {series.map((s) => (
          <polyline key={s.name} fill="none" stroke={s.color} strokeWidth="2.5" strokeLinejoin="round"
            points={s.ys.map((y, i) => `${X(s.xs ? s.xs[i] : i)},${Y(y)}`).join(' ')} />
        ))}
      </svg>
      <figcaption className="mt-2 flex gap-4 text-xs text-muted-foreground">
        {series.map((s) => <span key={s.name} className="flex items-center gap-2"><span className="h-0.5 w-4" style={{ background: s.color }} />{s.name}</span>)}
      </figcaption>
    </figure>
  );
}
