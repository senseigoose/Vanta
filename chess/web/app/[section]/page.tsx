import { notFound } from 'next/navigation';
import { PLACEHOLDERS } from '@/lib/sections';

export function generateStaticParams() {
  return Object.keys(PLACEHOLDERS).map((section) => ({ section }));
}

export default async function Page({ params }: { params: Promise<{ section: string }> }) {
  const { section } = await params;
  const p = PLACEHOLDERS[section];
  if (!p) notFound();
  return (
    <div className="max-w-[60ch]">
      <h1 className="text-2xl font-semibold tracking-tight">{p.title}</h1>
      <p className="mt-1 text-sm text-muted-foreground">{p.intro}</p>
      <h2 className="mt-6 text-sm font-semibold">This page will show</h2>
      <ul className="mt-2 list-disc pl-5 text-sm">{p.shows.map((s) => <li key={s}>{s}</li>)}</ul>
      <p className="mt-6 text-xs text-muted-foreground">Data comes from {p.source}. Not built yet.</p>
    </div>
  );
}
