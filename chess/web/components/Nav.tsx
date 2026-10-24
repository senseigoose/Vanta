'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Dumbbell, Gauge, Network, ScrollText, Search, Swords, Trophy } from 'lucide-react';
import { SECTIONS } from '@/lib/sections';
import { CURRENT_NET, VERSIONS } from '@/lib/facts';

const ICONS = [Swords, Trophy, Dumbbell, Gauge, Search, Network, ScrollText];

export default function Nav() {
  const path = usePathname();
  return (
    <aside className="hidden w-60 shrink-0 flex-col border-r border-border bg-[var(--color-sidebar)] p-4 md:flex">
      <nav aria-label="Sections" className="flex flex-col gap-1">
        {SECTIONS.map((s, i) => {
          const Icon = ICONS[i];
          const on = path === s.href;
          return (
            <Link key={s.href} href={s.href} aria-current={on ? 'page' : undefined}
              className={`flex h-10 items-center gap-3 rounded-md px-3 text-sm transition-colors focus-visible:outline-2 focus-visible:outline-primary ${on ? 'bg-primary/15 text-primary' : 'text-foreground/75 hover:bg-muted'}`}>
              <Icon className="size-4" />{s.label}
            </Link>
          );
        })}
      </nav>
      <div className="mt-8">
        <div className="mb-3 px-3 text-[11px] font-semibold tracking-widest text-muted-foreground">CURRENT NET</div>
        <div className="rounded-lg border border-border bg-muted/40 p-3 text-sm">
          <div>{CURRENT_NET}</div>
          <div className="mt-1 text-xs text-muted-foreground">Engine {VERSIONS[0].tag}</div>
        </div>
      </div>
    </aside>
  );
}
