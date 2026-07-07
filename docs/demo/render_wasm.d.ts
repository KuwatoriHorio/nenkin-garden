/* tslint:disable */
/* eslint-disable */

export class Sim {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * エージェント位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（render-005）。
     * `state.ax/ay` を読むだけ・非侵襲（sugar_positions と同型）。
     */
    agent_positions(): Float32Array;
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
     * 採餌モードのホーム座標（グリッド座標）。JS がホーム印を描くのに使う。
     * 従来モードでは一様散布のため参考値（重心近傍）。
     */
    home_x(): number;
    home_y(): number;
    /**
     * 採餌モードか（凝集初期化が有効か）。
     */
    is_forage(): boolean;
    /**
     * seed から新しいシミュレーションを作る（既定 params・既定の合成列島）。
     */
    constructor(seed: number);
    /**
     * core-002 採餌モード: ホームに凝集して始まり、trail 勾配コホージョンで
     * 群れがまとまったまま砂糖へ触手を伸ばす（伸び）。砂糖を消すと退縮する（縮み）。
     * core は同一・パラメータ既定値を変えるだけ（core ← render の一方向依存は不変）。
     */
    static new_forage(seed: number): Sim;
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
     * `show_trail=false` のとき陸/海の地形色のみを描き、trail の緑グロウは描かない
     * （render-005: エージェント可視化と併せて trail 非表示を選べるようにする render 側の表示切替。
     * State を読む範囲・描画専用ロジックのみで、core の力学には触れない）。
     */
    render(show_trail: boolean): void;
    /**
     * 実行中 Sim の回収レート（バイオマス増加量）を実行時に変更する（render-005・開発用チューニング）。
     * `params.rs` の既定値は変えない。core の力学（`step`）自体は不変で、次 tick から
     * この値を読む（決定性契約は「同一 params・同一入力→同一hash」のまま保たれる）。
     */
    set_collect_rate(v: number): void;
    /**
     * 実行中 Sim の trail 濃度上限（ソフト飽和）を実行時に変更する（render-006・開発用チューニング）。
     * `params.rs` の既定値（`f64::INFINITY`=上限なし）は変えない。core の力学（`step`）自体は不変で、
     * 次 tick から `params.trail_max` を読む（同型: `set_collect_rate` と同じ契約）。
     * `v` に `f64::INFINITY` を渡せば上限なしに戻せる（JS 側で `Infinity` を渡す想定）。
     */
    set_trail_max(v: number): void;
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
    readonly sim_agent_positions: (a: number) => [number, number];
    readonly sim_compute_graph: (a: number) => void;
    readonly sim_graph_edge_comp: (a: number) => [number, number];
    readonly sim_graph_edge_currents: (a: number) => [number, number];
    readonly sim_graph_edge_mst: (a: number) => [number, number];
    readonly sim_graph_edges: (a: number) => [number, number];
    readonly sim_graph_max_current: (a: number) => number;
    readonly sim_graph_nodes: (a: number) => [number, number];
    readonly sim_height: (a: number) => number;
    readonly sim_home_x: (a: number) => number;
    readonly sim_home_y: (a: number) => number;
    readonly sim_is_forage: (a: number) => number;
    readonly sim_new: (a: number) => number;
    readonly sim_new_forage: (a: number) => number;
    readonly sim_pixels_len: (a: number) => number;
    readonly sim_pixels_ptr: (a: number) => number;
    readonly sim_place_sugar_at_canvas: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly sim_remove_sugar_at_canvas: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly sim_render: (a: number, b: number) => void;
    readonly sim_set_collect_rate: (a: number, b: number) => void;
    readonly sim_set_trail_max: (a: number, b: number) => void;
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
