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
