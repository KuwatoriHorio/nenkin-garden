/* @ts-self-types="./render_wasm.d.ts" */

/**
 * render-net-001: 網 Physarum モデル（`nenkin_garden::netphys`）を wasm で駆動する render レイヤ。
 * `Sim`（Jones モデル）・`TreeSim`（成長木モデル）とは別 struct・別ページ（`docs/demo-net/`）専用。
 * 網モデルの力学（`netphys_step`）・`NetParams` 既定は変えない・駆動して読むだけ
 * （core ← render の一方向依存）。
 */
export class NetSim {
    static __wrap(ptr) {
        const obj = Object.create(NetSim.prototype);
        obj.__wbg_ptr = ptr;
        NetSimFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        NetSimFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_netsim_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    height() {
        const ret = wasm.netsim_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * ホーム座標（グリッド座標）。根ノード(0)の初期位置と一致する。
     * @returns {number}
     */
    home_x() {
        const ret = wasm.netsim_home_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    home_y() {
        const ret = wasm.netsim_home_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * 各辺のコンダクタンス D（管の太さ, Tero 刈り込みで太さが背骨へ集約される）を
     * `net_edges()` と同順で返す。`state.edges` を読むだけ・非侵襲。
     * @returns {Float32Array}
     */
    net_edge_widths() {
        const ret = wasm.netsim_net_edge_widths(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 網の辺を flat 配列 [a0,b0,a1,b1,...]（ノード index ペア, a<b）で返す（一般グラフ・ループ可）。
     * `state.edges` を読むだけ・非侵襲。`net_edge_widths()` と同順。
     * @returns {Uint32Array}
     */
    net_edges() {
        const ret = wasm.netsim_net_edges(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 網ノードの座標を flat 配列 [x0,y0,x1,y1,...]（グリッド座標, index 昇順）で返す。
     * `state.nodes` を読むだけ・非侵襲。
     * @returns {Float32Array}
     */
    net_nodes() {
        const ret = wasm.netsim_net_nodes(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 決定性検証用: 現在 NetState の 64bit ハッシュを16進文字列で返す。
     * @returns {string}
     */
    net_state_hash_hex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.netsim_net_state_hash_hex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * seed から新しい網 Physarum シミュレーションを作る（既定 NetParams・既存合成列島を共有）。
     * wasm_bindgen コンストラクタと衝突しないよう、関連関数として公開する（`TreeSim::new_tree` と同型）。
     * @param {number} seed
     * @returns {NetSim}
     */
    static new_net(seed) {
        const ret = wasm.netsim_new_net(seed);
        return NetSim.__wrap(ret);
    }
    /**
     * @returns {number}
     */
    pixels_len() {
        const ret = wasm.netsim_pixels_len(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
     * @returns {number}
     */
    pixels_ptr() {
        const ret = wasm.netsim_pixels_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * canvas クリック → セル → place_sugar（陸のみ）。置けたら true（`Sim`/`TreeSim` と同型）。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} strength
     * @returns {boolean}
     */
    place_sugar_at_canvas(cx, cy, cw, ch, strength) {
        const ret = wasm.netsim_place_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, strength);
        return ret !== 0;
    }
    /**
     * canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} radius
     * @returns {boolean}
     */
    remove_sugar_at_canvas(cx, cy, cw, ch, radius) {
        const ret = wasm.netsim_remove_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, radius);
        return ret !== 0;
    }
    /**
     * 現在 State を RGBA バッファへ地形（陸/海）のみ描画する（trail 場は無い）。
     * State は読むだけ・非侵襲（`TreeSim::render` と同型）。
     */
    render() {
        wasm.netsim_render(this.__wbg_ptr);
    }
    /**
     * 実行中 NetSim の consolidation 周期 `period_n` を実行時に変更する（render-net-002・観察用コントロール）。
     * `src/netphys/state.rs` の既定値（12）・`netphys_step` の力学は変えない（読み替えのみ）。
     * `v` は 1〜200 にクランプ（0 は consolidation が毎 tick 発火/ゼロ除算になりうるため下限を設ける）。
     * `set_collect_rate`/`set_w_rand` と同型（読むだけ＋params書換のみ・次 tick から反映）。
     * @param {number} v
     */
    set_period_n(v) {
        wasm.netsim_set_period_n(this.__wbg_ptr, v);
    }
    /**
     * 実行中 NetSim の同心リング probe の周期 `ring_period` を実行時に変更する
     * （render-net-004・観察用コントロール）。`src/netphys/state.rs` の既定値（0＝オフ）・
     * `netphys_step` の力学は変えない（読み替えのみ）。`v` は 0〜60 にクランプ整数化
     * （0 はリング機能オフ）。`set_period_n`/`set_w_elev` と同型。
     * @param {number} v
     */
    set_ring_period(v) {
        wasm.netsim_set_ring_period(this.__wbg_ptr, v);
    }
    /**
     * 実行中 NetSim の同心リング probe の到達距離 `ring_reach` を実行時に変更する
     * （render-net-004・観察用コントロール）。`src/netphys/state.rs` の既定値（6.0）・
     * `netphys_step` の力学は変えない（読み替えのみ）。`v` は 1.0〜30.0 にクランプ。
     * `set_period_n`/`set_w_elev` と同型（読むだけ＋params書換のみ・次 tick から反映）。
     * @param {number} v
     */
    set_ring_reach(v) {
        wasm.netsim_set_ring_reach(this.__wbg_ptr, v);
    }
    /**
     * 実行中 NetSim の標高忌避の強さ `w_elev` を実行時に変更する（render-net-003・観察用コントロール）。
     * `src/netphys/state.rs` の既定値（2.0）・`netphys_step` の力学は変えない（読み替えのみ）。
     * `v` は 0.0〜8.0 にクランプ（負値は 0＝方向バイアス無し、上限8で壁化を避ける）。
     * `set_period_n`/`set_collect_rate`/`set_w_rand` と同型（読むだけ＋params書換のみ・次 tick から反映）。
     * @param {number} v
     */
    set_w_elev(v) {
        wasm.netsim_set_w_elev(this.__wbg_ptr, v);
    }
    /**
     * 実行中 NetSim の放射スポークバイアスの強さ `w_radial` を実行時に変更する
     * （render-net-004・観察用コントロール）。`src/netphys/state.rs` の既定値（0.0）・
     * `netphys_step` の力学は変えない（読み替えのみ）。`v` は 0.0〜8.0 にクランプ
     * （負値は 0＝バイアス無し、上限8で壁化を避ける）。
     * `set_period_n`/`set_w_elev` と同型（読むだけ＋params書換のみ・次 tick から反映）。
     * @param {number} v
     */
    set_w_radial(v) {
        wasm.netsim_set_w_radial(this.__wbg_ptr, v);
    }
    /**
     * 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから netphys_step する（決定性）。
     */
    step() {
        wasm.netsim_step(this.__wbg_ptr);
    }
    /**
     * 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す。
     * @returns {Float32Array}
     */
    sugar_positions() {
        const ret = wasm.netsim_sugar_positions(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    tick() {
        const ret = wasm.netsim_tick(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    width() {
        const ret = wasm.netsim_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) NetSim.prototype[Symbol.dispose] = NetSim.prototype.free;

export class Sim {
    static __wrap(ptr) {
        const obj = Object.create(Sim.prototype);
        obj.__wbg_ptr = ptr;
        SimFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SimFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_sim_free(ptr, 0);
    }
    /**
     * 近接エージェントを結ぶリンクを flat `[a0,b0,a1,b1,...]`（エージェント index ペア, a<b）で
     * 返す（render-007）。グリッド空間分割で近傍探索し O(n) 程度に抑える。各エージェントは
     * 半径 `radius` 内の最近傍**最大2本**にのみ結ぶ（半径内全結合の密網にはしない）ことで、
     * ニューロン様の枝分かれした樹状に見せる。出力は (a,b) 昇順ソート・重複排除・自己リンク無し
     * の決定論的順序。`state.ax/ay` を読むだけ・非侵襲（`agent_positions` と同型）。
     * @param {number} radius
     * @returns {Uint32Array}
     */
    agent_links(radius) {
        const ret = wasm.sim_agent_links(this.__wbg_ptr, radius);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * エージェント位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（render-005）。
     * `state.ax/ay` を読むだけ・非侵襲（sugar_positions と同型）。
     * @returns {Float32Array}
     */
    agent_positions() {
        const ret = wasm.sim_agent_positions(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 現在 State のグラフ幾何を解析して内部キャッシュへ格納する（読み取りのみ・非侵襲）。
     * JS はこの後アクセサで配列を取得して canvas に描く（render-003）。
     */
    compute_graph() {
        wasm.sim_compute_graph(this.__wbg_ptr);
    }
    /**
     * @returns {Uint32Array}
     */
    graph_edge_comp() {
        const ret = wasm.sim_graph_edge_comp(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Float32Array}
     */
    graph_edge_currents() {
        const ret = wasm.sim_graph_edge_currents(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint8Array}
     */
    graph_edge_mst() {
        const ret = wasm.sim_graph_edge_mst(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    graph_edges() {
        const ret = wasm.sim_graph_edges(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    graph_max_current() {
        const ret = wasm.sim_graph_max_current(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Float32Array}
     */
    graph_nodes() {
        const ret = wasm.sim_graph_nodes(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    height() {
        const ret = wasm.sim_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * 採餌モードのホーム座標（グリッド座標）。JS がホーム印を描くのに使う。
     * 従来モードでは一様散布のため参考値（重心近傍）。
     * @returns {number}
     */
    home_x() {
        const ret = wasm.sim_home_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    home_y() {
        const ret = wasm.sim_home_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * 採餌モードか（凝集初期化が有効か）。
     * @returns {boolean}
     */
    is_forage() {
        const ret = wasm.sim_is_forage(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * seed から新しいシミュレーションを作る（既定 params・既定の合成列島）。
     * @param {number} seed
     */
    constructor(seed) {
        const ret = wasm.sim_new(seed);
        this.__wbg_ptr = ret;
        SimFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * core-002 採餌モード: ホームに凝集して始まり、trail 勾配コホージョンで
     * 群れがまとまったまま砂糖へ触手を伸ばす（伸び）。砂糖を消すと退縮する（縮み）。
     * core は同一・パラメータ既定値を変えるだけ（core ← render の一方向依存は不変）。
     * @param {number} seed
     * @returns {Sim}
     */
    static new_forage(seed) {
        const ret = wasm.sim_new_forage(seed);
        return Sim.__wrap(ret);
    }
    /**
     * @returns {number}
     */
    pixels_len() {
        const ret = wasm.sim_pixels_len(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
     * @returns {number}
     */
    pixels_ptr() {
        const ret = wasm.sim_pixels_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * canvas クリック → セル → place_sugar（陸のみ）。置けたら true。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} strength
     * @returns {boolean}
     */
    place_sugar_at_canvas(cx, cy, cw, ch, strength) {
        const ret = wasm.sim_place_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, strength);
        return ret !== 0;
    }
    /**
     * canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} radius
     * @returns {boolean}
     */
    remove_sugar_at_canvas(cx, cy, cw, ch, radius) {
        const ret = wasm.sim_remove_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, radius);
        return ret !== 0;
    }
    /**
     * 現在 State を RGBA バッファへ描画する（State は読むだけ・非侵襲）。
     * `show_trail=false` のとき陸/海の地形色のみを描き、trail の緑グロウは描かない
     * （render-005: エージェント可視化と併せて trail 非表示を選べるようにする render 側の表示切替。
     * State を読む範囲・描画専用ロジックのみで、core の力学には触れない）。
     * @param {boolean} show_trail
     */
    render(show_trail) {
        wasm.sim_render(this.__wbg_ptr, show_trail);
    }
    /**
     * 実行中 Sim の回収レート（バイオマス増加量）を実行時に変更する（render-005・開発用チューニング）。
     * `params.rs` の既定値は変えない。core の力学（`step`）自体は不変で、次 tick から
     * この値を読む（決定性契約は「同一 params・同一入力→同一hash」のまま保たれる）。
     * @param {number} v
     */
    set_collect_rate(v) {
        wasm.sim_set_collect_rate(this.__wbg_ptr, v);
    }
    /**
     * 実行中 Sim の trail 濃度上限（ソフト飽和）を実行時に変更する（render-006・開発用チューニング）。
     * `params.rs` の既定値（`f64::INFINITY`=上限なし）は変えない。core の力学（`step`）自体は不変で、
     * 次 tick から `params.trail_max` を読む（同型: `set_collect_rate` と同じ契約）。
     * `v` に `f64::INFINITY` を渡せば上限なしに戻せる（JS 側で `Infinity` を渡す想定）。
     * @param {number} v
     */
    set_trail_max(v) {
        wasm.sim_set_trail_max(this.__wbg_ptr, v);
    }
    /**
     * 決定性検証用: 現在 State の 64bit ハッシュを16進文字列で返す。
     * @returns {string}
     */
    state_hash_hex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.sim_state_hash_hex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから step する（決定性）。
     */
    step() {
        wasm.sim_step(this.__wbg_ptr);
    }
    /**
     * 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（JS が赤点を描く）。
     * @returns {Float32Array}
     */
    sugar_positions() {
        const ret = wasm.sim_sugar_positions(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    tick() {
        const ret = wasm.sim_tick(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    width() {
        const ret = wasm.sim_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) Sim.prototype[Symbol.dispose] = Sim.prototype.free;

/**
 * render-tree-001: 成長木モデル（`nenkin_garden::tree`）を wasm で駆動する render レイヤ。
 * `Sim`（Jones モデル）とは別 struct・別ページ（`docs/demo-tree/`）専用。
 * 木モデルの力学（`tree_step`）は変えない・駆動して読むだけ（core ← render の一方向依存）。
 */
export class TreeSim {
    static __wrap(ptr) {
        const obj = Object.create(TreeSim.prototype);
        obj.__wbg_ptr = ptr;
        TreeSimFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TreeSimFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_treesim_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    height() {
        const ret = wasm.treesim_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * ホーム座標（グリッド座標）。根ノードの初期位置と一致する。
     * @returns {number}
     */
    home_x() {
        const ret = wasm.treesim_home_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    home_y() {
        const ret = wasm.treesim_home_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * seed から新しい成長木シミュレーションを作る（既定 TreeParams・World は既存合成列島を共有）。
     * wasm_bindgen コンストラクタと衝突しないよう、関連関数として公開する。
     * @param {number} seed
     * @returns {TreeSim}
     */
    static new_tree(seed) {
        const ret = wasm.treesim_new_tree(seed);
        return TreeSim.__wrap(ret);
    }
    /**
     * @returns {number}
     */
    pixels_len() {
        const ret = wasm.treesim_pixels_len(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
     * @returns {number}
     */
    pixels_ptr() {
        const ret = wasm.treesim_pixels_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * canvas クリック → セル → place_sugar（陸のみ）。置けたら true（`Sim` と同型）。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} strength
     * @returns {boolean}
     */
    place_sugar_at_canvas(cx, cy, cw, ch, strength) {
        const ret = wasm.treesim_place_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, strength);
        return ret !== 0;
    }
    /**
     * canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
     * @param {number} cx
     * @param {number} cy
     * @param {number} cw
     * @param {number} ch
     * @param {number} radius
     * @returns {boolean}
     */
    remove_sugar_at_canvas(cx, cy, cw, ch, radius) {
        const ret = wasm.treesim_remove_sugar_at_canvas(this.__wbg_ptr, cx, cy, cw, ch, radius);
        return ret !== 0;
    }
    /**
     * 現在 State を RGBA バッファへ地形（陸/海）のみ描画する（trail 場は無い）。
     * State は読むだけ・非侵襲。
     */
    render() {
        wasm.treesim_render(this.__wbg_ptr);
    }
    /**
     * 実行中 TreeSim の探索方向の持続性（既定 0.45）を実行時に変更する（render-tree-002・任意の微調整用）。
     * `src/tree/state.rs` の既定値は変えない。`set_w_rand` と同型（読むだけ＋params書換のみ）。
     * @param {number} v
     */
    set_explore_persistence(v) {
        wasm.treesim_set_explore_persistence(this.__wbg_ptr, v);
    }
    /**
     * 実行中 TreeSim の探索強度（ランダム伸長）を実行時に変更する
     * （render-tree-002・開発用チューニング）。`src/tree/state.rs` の既定値（`w_rand=0.0`=探索オフ）
     * は変えない。木の力学（`tree_step`）自体は不変で、次 tick からこの値を読む
     * （決定性契約は「同一 params・同一入力→同一hash」のまま保たれる。`Sim::set_collect_rate` と同型）。
     * @param {number} v
     */
    set_w_rand(v) {
        wasm.treesim_set_w_rand(this.__wbg_ptr, v);
    }
    /**
     * 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから tree_step する（決定性）。
     */
    step() {
        wasm.treesim_step(this.__wbg_ptr);
    }
    /**
     * 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す。
     * @returns {Float32Array}
     */
    sugar_positions() {
        const ret = wasm.treesim_sugar_positions(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    tick() {
        const ret = wasm.treesim_tick(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * 親子パスを flat 配列 [child0,parent0,child1,parent1,...]（node index ペア）で返す。
     * 根（parent=None）は辺を持たない。木は単一連結・閉路なしなので
     * 辺数 == ノード数-1（`n_nodes()>=1` を前提, index 昇順で決定的）。
     * @returns {Uint32Array}
     */
    tree_edges() {
        const ret = wasm.treesim_tree_edges(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 木ノードの座標を flat 配列 [x0,y0,x1,y1,...]（グリッド座標, index 昇順）で返す。
     * `state.nodes` を読むだけ・非侵襲。
     * @returns {Float32Array}
     */
    tree_nodes() {
        const ret = wasm.treesim_tree_nodes(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * 決定性検証用: 現在 TreeState の 64bit ハッシュを16進文字列で返す。
     * @returns {string}
     */
    tree_state_hash_hex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.treesim_tree_state_hash_hex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    width() {
        const ret = wasm.treesim_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) TreeSim.prototype[Symbol.dispose] = TreeSim.prototype.free;
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_344f42d3211c4765: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./render_wasm_bg.js": import0,
    };
}

const NetSimFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_netsim_free(ptr, 1));
const SimFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_sim_free(ptr, 1));
const TreeSimFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_treesim_free(ptr, 1));

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('render_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
