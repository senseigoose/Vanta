import './globals.css';
import type { ReactNode } from 'react';
import Nav from '@/components/Nav';
import { CURRENT_NET, VERSIONS } from '@/lib/facts';

export const metadata = { title: 'Vanta', description: 'A chess engine you can play, analyze and inspect.' };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header className="flex h-16 items-center justify-between border-b border-border bg-card px-6">
          <div className="flex items-center gap-6">
            <div className="flex items-center gap-3">
              <div className="flex size-8 items-center justify-center rounded-md bg-primary text-lg text-primary-foreground" aria-hidden>♞</div>
              <span className="text-lg font-semibold tracking-tight">Vanta</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span className="size-2 rounded-full bg-amber-400" />Stub engine (WASM build pending)
            </div>
          </div>
          <span className="rounded-md border border-border px-3 py-1.5 text-sm">{VERSIONS[0].tag} · {CURRENT_NET}</span>
        </header>
        <div className="flex min-h-[calc(100vh-4rem)]">
          <Nav />
          <main className="min-w-0 flex-1 p-6">{children}</main>
        </div>
      </body>
    </html>
  );
}
