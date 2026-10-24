import type { ComponentProps } from 'react';

const cx = (...a: (string | false | undefined)[]) => a.filter(Boolean).join(' ');

export function Card({ className, ...p }: ComponentProps<'div'>) {
  return <div className={cx('rounded-xl border border-border bg-card', className)} {...p} />;
}

export function Button({ variant = 'outline', className, ...p }: ComponentProps<'button'> & { variant?: 'solid' | 'outline' | 'ghost' }) {
  const v = {
    solid: 'border-transparent bg-primary text-primary-foreground hover:bg-primary/90',
    outline: 'border-border bg-background hover:bg-muted',
    ghost: 'border-transparent hover:bg-muted',
  }[variant];
  return <button className={cx('inline-flex h-9 items-center justify-center gap-2 rounded-md border px-3 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-primary disabled:pointer-events-none disabled:opacity-40', v, className)} {...p} />;
}

export function Head({ title, sub }: { title: string; sub: string }) {
  return (
    <div className="mb-6">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="mt-1 text-sm text-muted-foreground">{sub}</p>
    </div>
  );
}

export function Table({ heads, rows }: { heads: string[]; rows: (string | number)[][] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-left text-xs">
        <thead className="bg-muted text-muted-foreground"><tr>{heads.map((h) => <th key={h} className="px-5 py-3 font-medium">{h}</th>)}</tr></thead>
        <tbody>{rows.map((r, i) => <tr key={i} className="border-t border-border">{r.map((c, j) => <td key={j} className={`px-5 py-3 ${j ? 'text-muted-foreground' : 'font-medium'}`}>{c}</td>)}</tr>)}</tbody>
      </table>
    </div>
  );
}
