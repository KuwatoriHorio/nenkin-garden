/* tslint:disable */
/* eslint-disable */

export class Sim {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * 現在 State のグラフ幾何を解析して内部キャッシュへ格納する（読み取りのみ・非侵襲）。
     * JS はこの後アクセサで配列を取得して canvas に描く（render-003）。
     */
    compute_graph(): void;
    graph_edge_comp(): Uint32Array;
    graph_edge_currents(): Float32Array;
    graph_edge_mst(): Uint8Array;
    graph_edges(): Uint32Array;
    graph_max_current(): number;
    graph_nodes(): Float32Array;
    height(): number;
    /**
     * seed から新しいシミュレーションを作る（既定 params・既定の合成列島）。
     */
    constructor(seed: number);
    pixels_len(): number;
    /**
     * RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
     */
    pixels_ptr(): number;
    /**
     * canvas クリック → セル → place_sugar（陸のみ）。置けたら true。
     */
    place_sugar_at_canvas(cx: number, cy: number, cw: number, ch: number, strength: number): boolean;
    /**
     * canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
     */
    remove_sugar_at_canvas(cx: number, cy: number, cw: number, ch: number, radius: number): boolean;
    /**
     * 現在 State を RGBA バッファへ描画する（State は読むだけ・非侵襲）。
     */
    render(): void;
    /**
     * 決定性検証用: 現在 State の 64bit ハッシュを16進文字列で返す。
     */
    state_hash_hex(): string;
    /**
     * 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから step する（決定性）。
     */
    step(): void;
    /**
     * 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（JS が赤点を描く）。
     */
    sugar_positions(): Float32Array;
    tick(): number;
    width(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_sim_free: (a: number, b: number) => void;
    readonly sim_compute_graph: (a: number) => void;
    readonly sim_graph_edge_comp: (a: number) => [number, number];
    readonly sim_graph_edge_currents: (a: number) => [number, number];
    readonly sim_graph_edge_mst: (a: number) => [number, number];
    readonly sim_graph_edges: (a: number) => [number, number];
    readonly sim_graph_max_current: (a: number) => number;
    readonly sim_graph_nodes: (a: number) => [number, number];
    readonly sim_height: (a: number) => number;
    readonly sim_new: (a: number) => number;
    readonly sim_pixels_len: (a: number) => number;
    readonly sim_pixels_ptr: (a: number) => number;
    readonly sim_place_sugar_at_canvas: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly sim_remove_sugar_at_canvas: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly sim_render: (a: number) => void;
    readonly sim_state_hash_hex: (a: number) => [number, number];
    readonly sim_step: (a: number) => void;
    readonly sim_sugar_positions: (a: number) => [number, number];
    readonly sim_tick: (a: number) => number;
    readonly sim_width: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
