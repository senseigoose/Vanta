/* tslint:disable */
/* eslint-disable */

export class Vanta {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Search the current position. A limit of 0 means "none". `on_info` is called with a JSON
     * string after every completed depth; the return value is a JSON string:
     * {"best":"e2e4","score":21,"mate":0,"depth":14,"nodes":123456}
     * `score` is in centipawns for the side to move; `mate` is non-zero only for forced
     * mates (moves to mate, negative if the side to move is being mated).
     */
    go(depth: number, nodes: number, movetime_ms: number, on_info: Function): string;
    /**
     * `net` is the contents of a .nnue file (for example vanta-v4.nnue).
     */
    constructor(net: Uint8Array, hash_mb: number);
    new_game(): void;
    /**
     * `fen` is a full FEN or the word "startpos"; `moves` are space-separated UCI moves
     * played from it (may be empty).
     */
    set_position(fen: string, moves: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_vanta_free: (a: number, b: number) => void;
    readonly vanta_go: (a: number, b: number, c: number, d: number, e: any) => [number, number];
    readonly vanta_new: (a: number, b: number, c: number) => [number, number, number];
    readonly vanta_new_game: (a: number) => void;
    readonly vanta_set_position: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
