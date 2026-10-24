// Reads Vanta's quantised net (768 -> 256 x2 -> 1) and evaluates a position, exactly as the engine does.
export const H = 256;
export const NIN = 768;
export const PARAMS = NIN * H + H + 2 * H + 1;
export type Net = { fw: Int16Array; fb: Int16Array; ow: Int16Array; ob: number };

export function parseNet(buf: ArrayBuffer): Net {
  const need = PARAMS * 2;
  if (buf.byteLength !== Math.ceil(need / 64) * 64) throw new Error('Not a 768→256×2→1 net');
  const v = new Int16Array(buf.slice(0, need));
  return { fw: v.subarray(0, NIN * H), fb: v.subarray(NIN * H, NIN * H + H), ow: v.subarray(NIN * H + H, NIN * H + 3 * H), ob: v[NIN * H + 3 * H] };
}

const feature = (persp: number, color: number, pc: number, sq: number) => (color === persp ? 0 : 384) + 64 * pc + (persp === 0 ? sq : sq ^ 56);

// Score in centipawns from the side to move's point of view.
export function evaluate(net: Net, fen: string): number {
  const [placement, side] = fen.trim().split(/\s+/);
  const acc = [Int16Array.from(net.fb), Int16Array.from(net.fb)];
  let rank = 7, file = 0;
  for (const ch of placement) {
    if (ch === '/') { rank--; file = 0; }
    else if (ch >= '1' && ch <= '8') file += Number(ch);
    else {
      const color = ch === ch.toUpperCase() ? 0 : 1;
      const pc = 'pnbrqk'.indexOf(ch.toLowerCase());
      const sq = rank * 8 + file++;
      for (let p = 0; p < 2; p++) {
        const base = feature(p, color, pc, sq) * H;
        for (let i = 0; i < H; i++) acc[p][i] += net.fw[base + i];
      }
    }
  }
  const stm = side === 'b' ? 1 : 0;
  const us = acc[stm], them = acc[stm ^ 1];
  const sc = (x: number) => { const y = Math.min(Math.max(x, 0), 255); return y * y; };
  let out = 0;
  for (let i = 0; i < H; i++) out += sc(us[i]) * net.ow[i] + sc(them[i]) * net.ow[H + i];
  out = Math.trunc(out / 255) + net.ob;
  return Math.trunc((out * 400) / (255 * 64));
}
