'use client';
import { useEffect, useRef } from 'react';
import { Chessground } from 'chessground';
import type { Api } from 'chessground/api';
import type { Key } from 'chessground/types';
import 'chessground/assets/chessground.base.css';
import 'chessground/assets/chessground.cburnett.css';

type Props = {
  fen: string;
  orientation: 'white' | 'black';
  turn: 'white' | 'black';
  check: boolean;
  lastMove?: [string, string];
  movable?: 'white' | 'black';
  dests: Map<Key, Key[]>;
  onMove: (from: string, to: string) => void;
};

export default function Board({ fen, orientation, turn, check, lastMove, movable, dests, onMove }: Props) {
  const el = useRef<HTMLDivElement>(null);
  const api = useRef<Api | null>(null);
  const cb = useRef(onMove);
  cb.current = onMove;

  useEffect(() => {
    if (!el.current) return;
    api.current = Chessground(el.current, { animation: { duration: 150 }, highlight: { lastMove: true, check: true } });
    return () => api.current?.destroy();
  }, []);

  useEffect(() => {
    api.current?.set({
      fen,
      orientation,
      turnColor: turn,
      check,
      lastMove: lastMove as Key[] | undefined,
      movable: { free: false, color: movable, dests, events: { after: (o, d) => cb.current(o, d) } },
    });
  }, [fen, orientation, turn, check, lastMove, movable, dests]);

  return <div className="board" ref={el} />;
}
