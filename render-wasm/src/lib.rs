//! render-001: 粘菌ガーデン ブラウザ対話デモ（WASM, render レイヤ）。
//!
//! 決定論コア（`nenkin_garden`）を wasm で実行し、trail 網を RGBA バッファへ描画する。
//! render は **State を読むだけ**（core ← render の一方向依存, 設計メモ §2）。
//! プレイヤーの動詞は §0 の「砂糖を置く／取り除く」「時間速度の変更」のみ。
//! 砂糖 op は tick 境界で適用し、決定性契約（§2/§4）を保つ。

use wasm_bindgen::prelude::*;

use nenkin_garden::analysis::analyze;
use nenkin_garden::graph_svg::mst_edge_set;
use nenkin_garden::netphys::{initial_net_state, netphys_state_hash, netphys_step, NetParams, NetState};
use nenkin_garden::params::Params;
use nenkin_garden::state::{apply_op, initial_state, Op, State};
use nenkin_garden::tree::{initial_tree_state, tree_state_hash, tree_step, TreeParams, TreeState};
use nenkin_garden::world::{make_synthetic_archipelago, World};
use nenkin_garden::{state_hash, step};

/// canvas 座標 (cx, cy) をグリッドセル (gx, gy) へ写像する純関数（native でテスト可能）。
/// 範囲外は端セルへクランプする。cw/ch は canvas 実ピクセル、gw/gh はグリッド寸法。
pub fn canvas_to_cell(cx: f64, cy: f64, cw: f64, ch: f64, gw: usize, gh: usize) -> (usize, usize) {
    let fx = if cw > 0.0 { (cx / cw) * gw as f64 } else { 0.0 };
    let fy = if ch > 0.0 { (cy / ch) * gh as f64 } else { 0.0 };
    let gx = (fx.floor().max(0.0) as usize).min(gw - 1);
    let gy = (fy.floor().max(0.0) as usize).min(gh - 1);
    (gx, gy)
}

fn lerp(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

fn land_color(e: f64) -> (f64, f64, f64) {
    let low = (46.0, 92.0, 60.0);
    let mid = (120.0, 104.0, 66.0);
    let high = (168.0, 162.0, 156.0);
    if e < 0.5 {
        lerp(low, mid, e / 0.5)
    } else {
        lerp(mid, high, (e - 0.5) / 0.5)
    }
}

#[wasm_bindgen]
pub struct Sim {
    world: World,
    params: Params,
    state: State,
    pending: Vec<Op>,
    pixels: Vec<u8>, // RGBA, グリッド解像度 (w*h*4)
    // グラフ幾何のキャッシュ（compute_graph で更新, render-003）
    gnodes: Vec<f32>, // flat [x,y,...] グリッド座標
    gedges: Vec<u32>, // flat [a,b,...] ノード index
    gcur: Vec<f32>,   // エッジ電流 |I_e|（gedges と同順）
    gmst: Vec<u8>,    // エッジが MST か（0/1）
    gcomp: Vec<u32>,  // エッジの連結成分 id
    gmaxcur: f32,     // 電流の最大（線幅正規化用）
}

#[wasm_bindgen]
impl Sim {
    /// seed から新しいシミュレーションを作る（既定 params・既定の合成列島）。
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32) -> Sim {
        Sim::build(seed, Params::default())
    }

    /// core-002 採餌モード: ホームに凝集して始まり、trail 勾配コホージョンで
    /// 群れがまとまったまま砂糖へ触手を伸ばす（伸び）。砂糖を消すと退縮する（縮み）。
    /// core は同一・パラメータ既定値を変えるだけ（core ← render の一方向依存は不変）。
    pub fn new_forage(seed: u32) -> Sim {
        let mut params = Params::default();
        params.init_cluster_sigma = 3.0;
        params.w_trail_cohesion = 1.0;
        // home_x/home_y は負のまま = World から低標高陸の重心近傍を自動選択（決定的）。
        Sim::build(seed, params)
    }

    fn build(seed: u32, params: Params) -> Sim {
        let world = make_synthetic_archipelago(&params);
        let state = initial_state(seed as u64, &world, &params);
        let pixels = vec![0u8; world.w * world.h * 4];
        Sim {
            world,
            params,
            state,
            pending: Vec::new(),
            pixels,
            gnodes: Vec::new(),
            gedges: Vec::new(),
            gcur: Vec::new(),
            gmst: Vec::new(),
            gcomp: Vec::new(),
            gmaxcur: 0.0,
        }
    }

    /// 採餌モードのホーム座標（グリッド座標）。JS がホーム印を描くのに使う。
    /// 従来モードでは一様散布のため参考値（重心近傍）。
    pub fn home_x(&self) -> f32 {
        self.world.default_home(self.params.e_lo).0 as f32
    }
    pub fn home_y(&self) -> f32 {
        self.world.default_home(self.params.e_lo).1 as f32
    }
    /// 採餌モードか（凝集初期化が有効か）。
    pub fn is_forage(&self) -> bool {
        self.params.init_cluster_sigma > 0.0
    }

    pub fn width(&self) -> usize {
        self.world.w
    }

    pub fn height(&self) -> usize {
        self.world.h
    }

    pub fn tick(&self) -> u32 {
        self.state.tick as u32
    }

    /// 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから step する（決定性）。
    pub fn step(&mut self) {
        let ops: Vec<Op> = std::mem::take(&mut self.pending);
        step(&mut self.state, &self.world, &self.params, &ops);
    }

    /// canvas クリック → セル → place_sugar（陸のみ）。置けたら true。
    pub fn place_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, strength: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        if !self.world.land_mask[gy * self.world.w + gx] {
            return false; // 海には置かない
        }
        self.pending.push(Op::PlaceSugar {
            x: gx as f64 + 0.5,
            y: gy as f64 + 0.5,
            strength,
        });
        true
    }

    /// canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
    pub fn remove_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, radius: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        let (px, py) = (gx as f64 + 0.5, gy as f64 + 0.5);
        let mut best: Option<(u64, f64)> = None;
        for i in 0..self.state.sugar_id.len() {
            let dx = self.state.sugar_x[i] - px;
            let dy = self.state.sugar_y[i] - py;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= radius && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((self.state.sugar_id[i], d));
            }
        }
        if let Some((id, _)) = best {
            self.pending.push(Op::RemoveSugar { id });
            true
        } else {
            false
        }
    }

    /// 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（JS が赤点を描く）。
    pub fn sugar_positions(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.sugar_x.len() * 2);
        for i in 0..self.state.sugar_x.len() {
            v.push(self.state.sugar_x[i] as f32);
            v.push(self.state.sugar_y[i] as f32);
        }
        v
    }

    /// エージェント位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す（render-005）。
    /// `state.ax/ay` を読むだけ・非侵襲（sugar_positions と同型）。
    pub fn agent_positions(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.ax.len() * 2);
        for i in 0..self.state.ax.len() {
            v.push(self.state.ax[i]);
            v.push(self.state.ay[i]);
        }
        v
    }

    /// 近接エージェントを結ぶリンクを flat `[a0,b0,a1,b1,...]`（エージェント index ペア, a<b）で
    /// 返す（render-007）。グリッド空間分割で近傍探索し O(n) 程度に抑える。各エージェントは
    /// 半径 `radius` 内の最近傍**最大2本**にのみ結ぶ（半径内全結合の密網にはしない）ことで、
    /// ニューロン様の枝分かれした樹状に見せる。出力は (a,b) 昇順ソート・重複排除・自己リンク無し
    /// の決定論的順序。`state.ax/ay` を読むだけ・非侵襲（`agent_positions` と同型）。
    pub fn agent_links(&self, radius: f64) -> Vec<u32> {
        let n = self.state.ax.len();
        if n == 0 || radius <= 0.0 {
            return Vec::new();
        }
        let cell = radius.max(1e-6);
        let cell_of = |x: f32, y: f32| -> (i64, i64) {
            ((x as f64 / cell).floor() as i64, (y as f64 / cell).floor() as i64)
        };
        // グリッド分割: セル座標 -> その中の agent index 一覧（index 昇順で挿入）。
        let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
        for i in 0..n {
            grid.entry(cell_of(self.state.ax[i], self.state.ay[i])).or_default().push(i);
        }
        let r2 = radius * radius;
        // BTreeSet で (a,b) を集約 → 挿入順に依存せず最終的に決定論的なソート済み集合になる。
        let mut pairs: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
        for i in 0..n {
            let (cx, cy) = cell_of(self.state.ax[i], self.state.ay[i]);
            let mut cand: Vec<(f64, usize)> = Vec::new();
            for dx in -1..=1i64 {
                for dy in -1..=1i64 {
                    if let Some(list) = grid.get(&(cx + dx, cy + dy)) {
                        for &j in list {
                            if j == i {
                                continue;
                            }
                            let ddx = (self.state.ax[i] - self.state.ax[j]) as f64;
                            let ddy = (self.state.ay[i] - self.state.ay[j]) as f64;
                            let d2 = ddx * ddx + ddy * ddy;
                            if d2 <= r2 {
                                cand.push((d2, j));
                            }
                        }
                    }
                }
            }
            // 距離昇順（タイは index 昇順）で安定ソート → 決定的。最近傍から最大2本まで採る。
            cand.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
            for &(_, j) in cand.iter().take(2) {
                let (a, b) = if i < j { (i as u32, j as u32) } else { (j as u32, i as u32) };
                pairs.insert((a, b));
            }
        }
        let mut out = Vec::with_capacity(pairs.len() * 2);
        for (a, b) in pairs {
            out.push(a);
            out.push(b);
        }
        out
    }

    /// 実行中 Sim の回収レート（バイオマス増加量）を実行時に変更する（render-005・開発用チューニング）。
    /// `params.rs` の既定値は変えない。core の力学（`step`）自体は不変で、次 tick から
    /// この値を読む（決定性契約は「同一 params・同一入力→同一hash」のまま保たれる）。
    pub fn set_collect_rate(&mut self, v: f64) {
        self.params.collect_rate = v;
    }

    /// 実行中 Sim の trail 濃度上限（ソフト飽和）を実行時に変更する（render-006・開発用チューニング）。
    /// `params.rs` の既定値（`f64::INFINITY`=上限なし）は変えない。core の力学（`step`）自体は不変で、
    /// 次 tick から `params.trail_max` を読む（同型: `set_collect_rate` と同じ契約）。
    /// `v` に `f64::INFINITY` を渡せば上限なしに戻せる（JS 側で `Infinity` を渡す想定）。
    pub fn set_trail_max(&mut self, v: f64) {
        self.params.trail_max = v;
    }

    /// 現在 State を RGBA バッファへ描画する（State は読むだけ・非侵襲）。
    /// `show_trail=false` のとき陸/海の地形色のみを描き、trail の緑グロウは描かない
    /// （render-005: エージェント可視化と併せて trail 非表示を選べるようにする render 側の表示切替。
    /// State を読む範囲・描画専用ロジックのみで、core の力学には触れない）。
    pub fn render(&mut self, show_trail: bool) {
        let (w, h) = (self.world.w, self.world.h);
        let maxt = self.state.trail.iter().cloned().fold(0.0f32, f32::max).max(1e-6) as f64;
        let glow = (124.0, 246.0, 152.0);
        for i in 0..w * h {
            let (r, g, b) = if self.world.land_mask[i] {
                let e = self.world.e[i] as f64;
                let base = land_color(e);
                if show_trail {
                    let t = (self.state.trail[i] as f64 / maxt).clamp(0.0, 1.0);
                    let a = (t * 1.6).min(1.0);
                    lerp(base, glow, a)
                } else {
                    base
                }
            } else {
                (11.0, 30.0, 45.0) // 海
            };
            let o = i * 4;
            self.pixels[o] = r as u8;
            self.pixels[o + 1] = g as u8;
            self.pixels[o + 2] = b as u8;
            self.pixels[o + 3] = 255;
        }
    }

    /// RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn pixels_len(&self) -> usize {
        self.pixels.len()
    }

    /// 決定性検証用: 現在 State の 64bit ハッシュを16進文字列で返す。
    pub fn state_hash_hex(&self) -> String {
        format!("{:016x}", state_hash(&self.state, &self.params))
    }

    /// 現在 State のグラフ幾何を解析して内部キャッシュへ格納する（読み取りのみ・非侵襲）。
    /// JS はこの後アクセサで配列を取得して canvas に描く（render-003）。
    pub fn compute_graph(&mut self) {
        let res = analyze(&self.state, &self.world, &self.params);
        let g = &res.graph;
        let w = self.world.w;
        self.gnodes = g
            .node_px
            .iter()
            .flat_map(|&pix| [((pix % w) as f32) + 0.5, ((pix / w) as f32) + 0.5])
            .collect();
        self.gedges = g
            .edges
            .iter()
            .flat_map(|e| [e.a as u32, e.b as u32])
            .collect();
        let has_flow = res.edge_currents.len() == g.edges.len();
        self.gcur = if has_flow {
            res.edge_currents.iter().map(|&c| c as f32).collect()
        } else {
            vec![0.0f32; g.edges.len()]
        };
        self.gmst = mst_edge_set(g).iter().map(|&b| b as u8).collect();
        self.gcomp = g.edges.iter().map(|e| g.node_comp[e.a] as u32).collect();
        self.gmaxcur = self.gcur.iter().cloned().fold(0.0f32, f32::max);
    }

    pub fn graph_nodes(&self) -> Vec<f32> {
        self.gnodes.clone()
    }
    pub fn graph_edges(&self) -> Vec<u32> {
        self.gedges.clone()
    }
    pub fn graph_edge_currents(&self) -> Vec<f32> {
        self.gcur.clone()
    }
    pub fn graph_edge_mst(&self) -> Vec<u8> {
        self.gmst.clone()
    }
    pub fn graph_edge_comp(&self) -> Vec<u32> {
        self.gcomp.clone()
    }
    pub fn graph_max_current(&self) -> f32 {
        self.gmaxcur
    }
}

/// render-tree-001: 成長木モデル（`nenkin_garden::tree`）を wasm で駆動する render レイヤ。
/// `Sim`（Jones モデル）とは別 struct・別ページ（`docs/demo-tree/`）専用。
/// 木モデルの力学（`tree_step`）は変えない・駆動して読むだけ（core ← render の一方向依存）。
#[wasm_bindgen]
pub struct TreeSim {
    world: World,
    params: TreeParams,
    state: TreeState,
    pending: Vec<Op>,
    pixels: Vec<u8>, // RGBA, グリッド解像度 (w*h*4)。地形（陸/海）のみ・trail 場は無い。
}

#[wasm_bindgen]
impl TreeSim {
    /// seed から新しい成長木シミュレーションを作る（既定 TreeParams・World は既存合成列島を共有）。
    /// wasm_bindgen コンストラクタと衝突しないよう、関連関数として公開する。
    pub fn new_tree(seed: u32) -> TreeSim {
        let world = make_synthetic_archipelago(&Params::default());
        let params = TreeParams::default();
        let state = initial_tree_state(seed as u64, &world, &params);
        let pixels = vec![0u8; world.w * world.h * 4];
        TreeSim { world, params, state, pending: Vec::new(), pixels }
    }

    /// ホーム座標（グリッド座標）。根ノードの初期位置と一致する。
    pub fn home_x(&self) -> f32 {
        self.world.default_home(self.params.e_lo).0 as f32
    }
    pub fn home_y(&self) -> f32 {
        self.world.default_home(self.params.e_lo).1 as f32
    }

    pub fn width(&self) -> usize {
        self.world.w
    }

    pub fn height(&self) -> usize {
        self.world.h
    }

    pub fn tick(&self) -> u32 {
        self.state.tick as u32
    }

    /// 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから tree_step する（決定性）。
    pub fn step(&mut self) {
        let ops: Vec<Op> = std::mem::take(&mut self.pending);
        tree_step(&mut self.state, &self.world, &self.params, &ops);
    }

    /// canvas クリック → セル → place_sugar（陸のみ）。置けたら true（`Sim` と同型）。
    pub fn place_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, strength: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        if !self.world.land_mask[gy * self.world.w + gx] {
            return false; // 海には置かない
        }
        self.pending.push(Op::PlaceSugar {
            x: gx as f64 + 0.5,
            y: gy as f64 + 0.5,
            strength,
        });
        true
    }

    /// canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
    pub fn remove_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, radius: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        let (px, py) = (gx as f64 + 0.5, gy as f64 + 0.5);
        let mut best: Option<(u64, f64)> = None;
        for i in 0..self.state.sugar_id.len() {
            let dx = self.state.sugar_x[i] - px;
            let dy = self.state.sugar_y[i] - py;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= radius && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((self.state.sugar_id[i], d));
            }
        }
        if let Some((id, _)) = best {
            self.pending.push(Op::RemoveSugar { id });
            true
        } else {
            false
        }
    }

    /// 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す。
    pub fn sugar_positions(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.sugar_x.len() * 2);
        for i in 0..self.state.sugar_x.len() {
            v.push(self.state.sugar_x[i] as f32);
            v.push(self.state.sugar_y[i] as f32);
        }
        v
    }

    /// 木ノードの座標を flat 配列 [x0,y0,x1,y1,...]（グリッド座標, index 昇順）で返す。
    /// `state.nodes` を読むだけ・非侵襲。
    pub fn tree_nodes(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.nodes.len() * 2);
        for n in &self.state.nodes {
            v.push(n.x);
            v.push(n.y);
        }
        v
    }

    /// 親子パスを flat 配列 [child0,parent0,child1,parent1,...]（node index ペア）で返す。
    /// 根（parent=None）は辺を持たない。木は単一連結・閉路なしなので
    /// 辺数 == ノード数-1（`n_nodes()>=1` を前提, index 昇順で決定的）。
    pub fn tree_edges(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.state.nodes.len().saturating_sub(1) * 2);
        for (i, n) in self.state.nodes.iter().enumerate() {
            if let Some(par) = n.parent {
                v.push(i as u32);
                v.push(par as u32);
            }
        }
        v
    }

    /// 決定性検証用: 現在 TreeState の 64bit ハッシュを16進文字列で返す。
    pub fn tree_state_hash_hex(&self) -> String {
        format!("{:016x}", tree_state_hash(&self.state, &self.params))
    }

    /// 実行中 TreeSim の探索強度（ランダム伸長）を実行時に変更する
    /// （render-tree-002・開発用チューニング）。`src/tree/state.rs` の既定値（`w_rand=0.0`=探索オフ）
    /// は変えない。木の力学（`tree_step`）自体は不変で、次 tick からこの値を読む
    /// （決定性契約は「同一 params・同一入力→同一hash」のまま保たれる。`Sim::set_collect_rate` と同型）。
    pub fn set_w_rand(&mut self, v: f64) {
        self.params.w_rand = v;
    }

    /// 実行中 TreeSim の探索方向の持続性（既定 0.45）を実行時に変更する（render-tree-002・任意の微調整用）。
    /// `src/tree/state.rs` の既定値は変えない。`set_w_rand` と同型（読むだけ＋params書換のみ）。
    pub fn set_explore_persistence(&mut self, v: f64) {
        self.params.explore_persistence = v;
    }

    /// 現在 State を RGBA バッファへ地形（陸/海）のみ描画する（trail 場は無い）。
    /// State は読むだけ・非侵襲。
    pub fn render(&mut self) {
        let (w, h) = (self.world.w, self.world.h);
        for i in 0..w * h {
            let (r, g, b) = if self.world.land_mask[i] {
                let e = self.world.e[i] as f64;
                land_color(e)
            } else {
                (11.0, 30.0, 45.0) // 海
            };
            let o = i * 4;
            self.pixels[o] = r as u8;
            self.pixels[o + 1] = g as u8;
            self.pixels[o + 2] = b as u8;
            self.pixels[o + 3] = 255;
        }
    }

    /// RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn pixels_len(&self) -> usize {
        self.pixels.len()
    }
}

/// render-net-001: 網 Physarum モデル（`nenkin_garden::netphys`）を wasm で駆動する render レイヤ。
/// `Sim`（Jones モデル）・`TreeSim`（成長木モデル）とは別 struct・別ページ（`docs/demo-net/`）専用。
/// 網モデルの力学（`netphys_step`）・`NetParams` 既定は変えない・駆動して読むだけ
/// （core ← render の一方向依存）。
#[wasm_bindgen]
pub struct NetSim {
    world: World,
    params: NetParams,
    state: NetState,
    pending: Vec<Op>,
    pixels: Vec<u8>, // RGBA, グリッド解像度 (w*h*4)。地形（陸/海）のみ・trail 場は無い。
}

#[wasm_bindgen]
impl NetSim {
    /// seed から新しい網 Physarum シミュレーションを作る（既定 NetParams・既存合成列島を共有）。
    /// wasm_bindgen コンストラクタと衝突しないよう、関連関数として公開する（`TreeSim::new_tree` と同型）。
    pub fn new_net(seed: u32) -> NetSim {
        let world = make_synthetic_archipelago(&Params::default());
        let params = NetParams::default();
        let state = initial_net_state(seed as u64, &world, &params);
        let pixels = vec![0u8; world.w * world.h * 4];
        NetSim { world, params, state, pending: Vec::new(), pixels }
    }

    /// ホーム座標（グリッド座標）。根ノード(0)の初期位置と一致する。
    pub fn home_x(&self) -> f32 {
        self.world.default_home(self.params.e_lo).0 as f32
    }
    pub fn home_y(&self) -> f32 {
        self.world.default_home(self.params.e_lo).1 as f32
    }

    pub fn width(&self) -> usize {
        self.world.w
    }

    pub fn height(&self) -> usize {
        self.world.h
    }

    pub fn tick(&self) -> u32 {
        self.state.tick as u32
    }

    /// 1 tick 進める。保留中の砂糖 op を tick 境界で適用してから netphys_step する（決定性）。
    pub fn step(&mut self) {
        let ops: Vec<Op> = std::mem::take(&mut self.pending);
        netphys_step(&mut self.state, &self.world, &self.params, &ops);
    }

    /// canvas クリック → セル → place_sugar（陸のみ）。置けたら true（`Sim`/`TreeSim` と同型）。
    pub fn place_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, strength: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        if !self.world.land_mask[gy * self.world.w + gx] {
            return false; // 海には置かない
        }
        self.pending.push(Op::PlaceSugar {
            x: gx as f64 + 0.5,
            y: gy as f64 + 0.5,
            strength,
        });
        true
    }

    /// canvas クリック近傍の砂糖源を1つ取り除く（半径 radius セル内で最近傍）。
    pub fn remove_sugar_at_canvas(&mut self, cx: f64, cy: f64, cw: f64, ch: f64, radius: f64) -> bool {
        let (gx, gy) = canvas_to_cell(cx, cy, cw, ch, self.world.w, self.world.h);
        let (px, py) = (gx as f64 + 0.5, gy as f64 + 0.5);
        let mut best: Option<(u64, f64)> = None;
        for i in 0..self.state.sugar_id.len() {
            let dx = self.state.sugar_x[i] - px;
            let dy = self.state.sugar_y[i] - py;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= radius && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((self.state.sugar_id[i], d));
            }
        }
        if let Some((id, _)) = best {
            self.pending.push(Op::RemoveSugar { id });
            true
        } else {
            false
        }
    }

    /// 砂糖源の位置を flat 配列 [x0,y0,x1,y1,...]（グリッド座標）で返す。
    pub fn sugar_positions(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.sugar_x.len() * 2);
        for i in 0..self.state.sugar_x.len() {
            v.push(self.state.sugar_x[i] as f32);
            v.push(self.state.sugar_y[i] as f32);
        }
        v
    }

    /// 網ノードの座標を flat 配列 [x0,y0,x1,y1,...]（グリッド座標, index 昇順）で返す。
    /// `state.nodes` を読むだけ・非侵襲。
    pub fn net_nodes(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.state.nodes.len() * 2);
        for n in &self.state.nodes {
            v.push(n.x as f32);
            v.push(n.y as f32);
        }
        v
    }

    /// 網の辺を flat 配列 [a0,b0,a1,b1,...]（ノード index ペア, a<b）で返す（一般グラフ・ループ可）。
    /// `state.edges` を読むだけ・非侵襲。`net_edge_widths()` と同順。
    pub fn net_edges(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.state.edges.len() * 2);
        for e in &self.state.edges {
            v.push(e.a as u32);
            v.push(e.b as u32);
        }
        v
    }

    /// 各辺のコンダクタンス D（管の太さ, Tero 刈り込みで太さが背骨へ集約される）を
    /// `net_edges()` と同順で返す。`state.edges` を読むだけ・非侵襲。
    pub fn net_edge_widths(&self) -> Vec<f32> {
        self.state.edges.iter().map(|e| e.d as f32).collect()
    }

    /// 決定性検証用: 現在 NetState の 64bit ハッシュを16進文字列で返す。
    pub fn net_state_hash_hex(&self) -> String {
        format!("{:016x}", netphys_state_hash(&self.state, &self.params))
    }

    /// 実行中 NetSim の consolidation 周期 `period_n` を実行時に変更する（render-net-002・観察用コントロール）。
    /// `src/netphys/state.rs` の既定値（12）・`netphys_step` の力学は変えない（読み替えのみ）。
    /// `v` は 1〜200 にクランプ（0 は consolidation が毎 tick 発火/ゼロ除算になりうるため下限を設ける）。
    /// `set_collect_rate`/`set_w_rand` と同型（読むだけ＋params書換のみ・次 tick から反映）。
    pub fn set_period_n(&mut self, v: f64) {
        let clamped = v.round().clamp(1.0, 200.0) as u64;
        self.params.period_n = clamped;
    }

    /// 実行中 NetSim の標高忌避の強さ `w_elev` を実行時に変更する（render-net-003・観察用コントロール）。
    /// `src/netphys/state.rs` の既定値（2.0）・`netphys_step` の力学は変えない（読み替えのみ）。
    /// `v` は 0.0〜8.0 にクランプ（負値は 0＝方向バイアス無し、上限8で壁化を避ける）。
    /// `set_period_n`/`set_collect_rate`/`set_w_rand` と同型（読むだけ＋params書換のみ・次 tick から反映）。
    pub fn set_w_elev(&mut self, v: f64) {
        self.params.w_elev = v.clamp(0.0, 8.0);
    }

    /// 現在 State を RGBA バッファへ地形（陸/海）のみ描画する（trail 場は無い）。
    /// State は読むだけ・非侵襲（`TreeSim::render` と同型）。
    pub fn render(&mut self) {
        let (w, h) = (self.world.w, self.world.h);
        for i in 0..w * h {
            let (r, g, b) = if self.world.land_mask[i] {
                let e = self.world.e[i] as f64;
                land_color(e)
            } else {
                (11.0, 30.0, 45.0) // 海
            };
            let o = i * 4;
            self.pixels[o] = r as u8;
            self.pixels[o + 1] = g as u8;
            self.pixels[o + 2] = b as u8;
            self.pixels[o + 3] = 255;
        }
    }

    /// RGBA バッファの先頭ポインタ（JS が wasm memory から読む）。
    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn pixels_len(&self) -> usize {
        self.pixels.len()
    }
}

/// 保留 op を適用せずに単純ステップする内部用（テスト・ヘッドレス比較用, wasm 非公開）。
pub fn apply_op_now(sim_state: &mut State, op: &Op) {
    apply_op(sim_state, op);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_to_cell_maps_and_clamps() {
        // 96x96 グリッドを 480x480 canvas に表示（5px/セル）
        assert_eq!(canvas_to_cell(0.0, 0.0, 480.0, 480.0, 96, 96), (0, 0));
        assert_eq!(canvas_to_cell(479.0, 479.0, 480.0, 480.0, 96, 96), (95, 95));
        // 中央付近
        assert_eq!(canvas_to_cell(240.0, 240.0, 480.0, 480.0, 96, 96), (48, 48));
        // 範囲外はクランプ
        assert_eq!(canvas_to_cell(-10.0, 999.0, 480.0, 480.0, 96, 96), (0, 95));
    }

    #[test]
    fn place_sugar_respects_land_and_is_deterministic() {
        let mut a = Sim::new(42);
        let mut b = Sim::new(42);
        // 同一操作列 → 同一 state_hash（tick境界適用・単一PRNG）
        for _ in 0..20 {
            a.step();
            b.step();
        }
        assert_eq!(a.state_hash_hex(), b.state_hash_hex());
        // render は State を書き換えない（前後で hash 不変）
        let h = a.state_hash_hex();
        a.render(true);
        assert_eq!(a.state_hash_hex(), h);
    }

    #[test]
    fn forage_mode_is_deterministic_and_clusters() {
        let mut a = Sim::new_forage(42);
        let mut b = Sim::new_forage(42);
        assert!(a.is_forage() && b.is_forage());
        // ホームは決定的
        assert_eq!(a.home_x(), b.home_x());
        assert_eq!(a.home_y(), b.home_y());
        // 初期エージェントはホーム近傍に凝集（従来 new は陸全体に散る）。
        let (hx, hy) = (a.home_x() as f64, a.home_y() as f64);
        let near = (0..a.state.n_agents())
            .filter(|&i| {
                let dx = a.state.ax[i] as f64 - hx;
                let dy = a.state.ay[i] as f64 - hy;
                (dx * dx + dy * dy).sqrt() <= 12.0
            })
            .count();
        assert!(
            near * 2 >= a.state.n_agents(),
            "forage init should cluster near home: {near}/{}",
            a.state.n_agents()
        );
        // 同一操作列 → 同一 state_hash（決定性）
        for _ in 0..30 {
            a.step();
            b.step();
        }
        assert_eq!(a.state_hash_hex(), b.state_hash_hex());
    }

    #[test]
    fn agent_positions_len_matches_and_is_non_invasive() {
        let mut a = Sim::new_forage(42);
        for _ in 0..10 {
            a.step();
        }
        let h = a.state_hash_hex();
        let pos = a.agent_positions();
        // 長さ: エージェント数 × 2（x,y）
        assert_eq!(pos.len(), 2 * a.state.n_agents());
        // 内容: state.ax/ay と一致（読むだけ）
        for i in 0..a.state.n_agents() {
            assert_eq!(pos[i * 2], a.state.ax[i]);
            assert_eq!(pos[i * 2 + 1], a.state.ay[i]);
        }
        // 取得前後で state_hash 不変（非侵襲）
        assert_eq!(a.state_hash_hex(), h);
    }

    #[test]
    fn agent_links_is_deterministic_and_non_invasive() {
        let mut a = Sim::new_forage(42);
        let mut b = Sim::new_forage(42);
        for _ in 0..15 {
            a.step();
            b.step();
        }
        let h = a.state_hash_hex();
        let la = a.agent_links(6.0);
        // 呼び出し前後で state_hash 不変（非侵襲）
        assert_eq!(a.state_hash_hex(), h);
        let lb = b.agent_links(6.0);
        // 同一 State → 同一リンク（決定論）
        assert_eq!(la, lb);
    }

    #[test]
    fn agent_links_are_valid_pairs() {
        let mut a = Sim::new_forage(7);
        for _ in 0..20 {
            a.step();
        }
        let n = a.state.n_agents() as u32;
        let links = a.agent_links(6.0);
        assert_eq!(links.len() % 2, 0, "flat pairs must have even length");
        let mut prev: Option<(u32, u32)> = None;
        for c in links.chunks(2) {
            let (x, y) = (c[0], c[1]);
            assert!(x < n && y < n, "indices must be within agent range");
            assert!(x < y, "each pair must be stored as a<b (no self-links)");
            let pair = (x, y);
            if let Some(p) = prev {
                assert!(p < pair, "pairs must be sorted ascending with no duplicates");
            }
            prev = Some(pair);
        }
    }

    #[test]
    fn agent_links_empty_radius_or_no_agents_is_empty() {
        let a = Sim::new(1);
        assert!(a.agent_links(0.0).is_empty());
        assert!(a.agent_links(-1.0).is_empty());
    }

    #[test]
    fn set_collect_rate_updates_params_and_affects_uptake() {
        let mut a = Sim::new(42);
        let before = a.params.collect_rate;
        a.set_collect_rate(before * 5.0 + 1.0);
        assert_eq!(a.params.collect_rate, before * 5.0 + 1.0);
        assert_ne!(a.params.collect_rate, before);

        // 挙動でも確認: 同一初期状態から、片方だけ回収レートを上げると
        // 同じ tick 数後の collected_total がより大きくなる（砂糖はホーム同座標に十分量置く）。
        let mut lo = Sim::new_forage(7);
        let mut hi = Sim::new_forage(7);
        let (hx, hy) = (lo.home_x() as f64, lo.home_y() as f64);
        lo.pending.push(Op::PlaceSugar { x: hx, y: hy, strength: 5000.0 });
        hi.pending.push(Op::PlaceSugar { x: hx, y: hy, strength: 5000.0 });
        let base = hi.params.collect_rate;
        hi.set_collect_rate(base * 4.0);
        for _ in 0..40 {
            lo.step();
            hi.step();
        }
        assert!(lo.state.collected_total > 0.0, "test setup should cause contact");
        assert!(hi.state.collected_total > lo.state.collected_total);
    }

    #[test]
    fn set_trail_max_updates_params_only() {
        let mut a = Sim::new(42);
        // 既定は core-004 の既定どおり「上限なし」（params.rs は変えていないことの確認）。
        assert_eq!(a.params.trail_max, f64::INFINITY);
        a.set_trail_max(18.0);
        assert_eq!(a.params.trail_max, 18.0);
        // 上限なしへ戻せる（JS 側で Infinity を渡す想定）。
        a.set_trail_max(f64::INFINITY);
        assert_eq!(a.params.trail_max, f64::INFINITY);
    }

    #[test]
    fn set_trail_max_is_non_invasive_and_deterministic() {
        // setter 呼び出し自体は state を書き換えない（読むだけ・非侵襲）。
        let mut a = Sim::new_forage(7);
        let h = a.state_hash_hex();
        a.set_trail_max(18.0);
        assert_eq!(a.state_hash_hex(), h);

        // 同一操作列・同一 trail_max → 同一 state_hash（決定性契約は保たれる）。
        let mut x = Sim::new_forage(7);
        let mut y = Sim::new_forage(7);
        x.set_trail_max(18.0);
        y.set_trail_max(18.0);
        for _ in 0..30 {
            x.step();
            y.step();
        }
        assert_eq!(x.state_hash_hex(), y.state_hash_hex());
    }

    // --- render-tree-001: TreeSim（成長木モデル）のnative test ---

    #[test]
    fn tree_sim_same_ops_yield_same_hash() {
        let mut a = TreeSim::new_tree(42);
        let mut b = TreeSim::new_tree(42);
        for i in 0..30 {
            if i == 5 {
                let (hx, hy) = (a.home_x() as f64, a.home_y() as f64);
                a.place_sugar_at_canvas(
                    (hx + 10.0) * 5.0,
                    hy * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    80.0,
                );
                let (hx2, hy2) = (b.home_x() as f64, b.home_y() as f64);
                b.place_sugar_at_canvas(
                    (hx2 + 10.0) * 5.0,
                    hy2 * 5.0,
                    b.world.w as f64 * 5.0,
                    b.world.h as f64 * 5.0,
                    80.0,
                );
            }
            a.step();
            b.step();
        }
        // 同一 seed・同一操作列 → 同一 tree_state_hash（決定性契約）
        assert_eq!(a.tree_state_hash_hex(), b.tree_state_hash_hex());

        // render は State を書き換えない（前後で hash 不変）
        let h = a.tree_state_hash_hex();
        a.render();
        assert_eq!(a.tree_state_hash_hex(), h);
    }

    #[test]
    fn tree_sim_edges_form_a_connected_tree() {
        let mut a = TreeSim::new_tree(7);
        let (hx, hy) = (a.home_x() as f64, a.home_y() as f64);
        for i in 0..40 {
            if i == 3 {
                a.place_sugar_at_canvas(
                    (hx + 15.0) * 5.0,
                    (hy - 8.0) * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    150.0,
                );
                a.place_sugar_at_canvas(
                    (hx - 12.0) * 5.0,
                    (hy + 14.0) * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    150.0,
                );
            }
            a.step();
        }
        let n_nodes = a.tree_nodes().len() / 2;
        assert!(n_nodes >= 1, "tree should have at least the root node");
        // 連結木・根1つ: edges.len() == 2*(nodes数-1)
        assert_eq!(a.tree_edges().len(), 2 * (n_nodes - 1));
    }

    #[test]
    fn set_w_rand_updates_params_and_is_non_invasive() {
        let mut a = TreeSim::new_tree(42);
        // 既定は探索オフ（src/tree/state.rs の既定値・変更していないことの確認）。
        assert_eq!(a.params.w_rand, 0.0);
        let h = a.tree_state_hash_hex();
        a.set_w_rand(0.3);
        assert_eq!(a.params.w_rand, 0.3);
        // setter 呼び出し自体は state を書き換えない（読むだけ・非侵襲）。
        assert_eq!(a.tree_state_hash_hex(), h);
        // 探索オフへ戻せる。
        a.set_w_rand(0.0);
        assert_eq!(a.params.w_rand, 0.0);

        // 挙動でも確認: 同一 seed・砂糖なしで w_rand>0 だと根から動きが生じる
        // （w_rand==0 だと砂糖が無い限りノードは増減しない/根に留まる）。
        let mut off = TreeSim::new_tree(7);
        let mut on = TreeSim::new_tree(7);
        on.set_w_rand(0.3);
        for _ in 0..40 {
            off.step();
            on.step();
        }
        assert_ne!(off.tree_state_hash_hex(), on.tree_state_hash_hex());
    }

    #[test]
    fn set_explore_persistence_updates_params_and_is_non_invasive() {
        let mut a = TreeSim::new_tree(42);
        assert_eq!(a.params.explore_persistence, 0.45);
        let h = a.tree_state_hash_hex();
        a.set_explore_persistence(0.8);
        assert_eq!(a.params.explore_persistence, 0.8);
        // setter 呼び出し自体は state を書き換えない（読むだけ・非侵襲）。
        assert_eq!(a.tree_state_hash_hex(), h);
    }

    #[test]
    fn compute_graph_is_deterministic_and_non_invasive() {
        let mut a = Sim::new(42);
        let mut b = Sim::new(42);
        for _ in 0..40 {
            a.step();
            b.step();
        }
        // compute_graph は State を書き換えない（前後で hash 不変）
        let h = a.state_hash_hex();
        a.compute_graph();
        assert_eq!(a.state_hash_hex(), h);
        b.compute_graph();
        // 同一 State → 同一グラフ幾何
        assert_eq!(a.graph_nodes(), b.graph_nodes());
        assert_eq!(a.graph_edges(), b.graph_edges());
        assert_eq!(a.graph_edge_currents(), b.graph_edge_currents());
        assert_eq!(a.graph_edge_mst(), b.graph_edge_mst());
        // 整合: 電流配列長 == エッジ数 == エッジ配列の半分
        assert_eq!(a.graph_edge_currents().len() * 2, a.graph_edges().len());
    }

    // --- render-net-001: NetSim（網 Physarum モデル）のnative test ---

    #[test]
    fn net_sim_same_ops_yield_same_hash() {
        let mut a = NetSim::new_net(42);
        let mut b = NetSim::new_net(42);
        for i in 0..80 {
            if i == 5 {
                let (hx, hy) = (a.home_x() as f64, a.home_y() as f64);
                a.place_sugar_at_canvas(
                    (hx + 10.0) * 5.0,
                    hy * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    80.0,
                );
                let (hx2, hy2) = (b.home_x() as f64, b.home_y() as f64);
                b.place_sugar_at_canvas(
                    (hx2 + 10.0) * 5.0,
                    hy2 * 5.0,
                    b.world.w as f64 * 5.0,
                    b.world.h as f64 * 5.0,
                    80.0,
                );
            }
            a.step();
            b.step();
        }
        // 同一 seed・同一操作列 → 同一 net_state_hash（決定性契約）
        assert_eq!(a.net_state_hash_hex(), b.net_state_hash_hex());

        // render は State を書き換えない（前後で hash 不変）
        let h = a.net_state_hash_hex();
        a.render();
        assert_eq!(a.net_state_hash_hex(), h);
    }

    #[test]
    fn net_edge_widths_len_matches_edges() {
        let mut a = NetSim::new_net(7);
        let (hx, hy) = (a.home_x() as f64, a.home_y() as f64);
        for i in 0..60 {
            if i == 3 {
                a.place_sugar_at_canvas(
                    (hx + 15.0) * 5.0,
                    (hy - 8.0) * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    150.0,
                );
                a.place_sugar_at_canvas(
                    (hx - 12.0) * 5.0,
                    (hy + 14.0) * 5.0,
                    a.world.w as f64 * 5.0,
                    a.world.h as f64 * 5.0,
                    150.0,
                );
            }
            a.step();
        }
        // 整合: edge_widths 長 == edges 長/2
        assert_eq!(a.net_edge_widths().len() * 2, a.net_edges().len());
        // ノードもある程度育っていること（探索が進んでいるかのスモークチェック）
        assert!(a.net_nodes().len() / 2 >= 1);
    }

    #[test]
    fn set_period_n_updates_params_and_is_non_invasive_and_deterministic() {
        let mut a = NetSim::new_net(42);
        // 既定は 12（src/netphys/state.rs の既定値・変更していないことの確認）。
        assert_eq!(a.params.period_n, 12);
        let h = a.net_state_hash_hex();
        a.set_period_n(30.0);
        assert_eq!(a.params.period_n, 30);
        // setter 呼び出し自体は state を書き換えない（読むだけ・非侵襲）。
        assert_eq!(a.net_state_hash_hex(), h);

        // クランプ: 下限1（0 以下は 1 に）・上限200（それ以上は 200 に）。
        a.set_period_n(0.0);
        assert_eq!(a.params.period_n, 1);
        a.set_period_n(-5.0);
        assert_eq!(a.params.period_n, 1);
        a.set_period_n(9999.0);
        assert_eq!(a.params.period_n, 200);

        // 非侵襲・決定性: 同一 seed・同一操作列・同一 period_n → 同一 net_state_hash。
        let mut x = NetSim::new_net(7);
        let mut y = NetSim::new_net(7);
        x.set_period_n(18.0);
        y.set_period_n(18.0);
        for _ in 0..60 {
            x.step();
            y.step();
        }
        assert_eq!(x.net_state_hash_hex(), y.net_state_hash_hex());
    }

    #[test]
    fn set_w_elev_updates_params_and_is_non_invasive_and_deterministic() {
        let mut a = NetSim::new_net(42);
        // 既定は 2.0（src/netphys/state.rs の既定値・変更していないことの確認）。
        assert_eq!(a.params.w_elev, 2.0);
        let h = a.net_state_hash_hex();
        a.set_w_elev(4.0);
        assert_eq!(a.params.w_elev, 4.0);
        // setter 呼び出し自体は state を書き換えない（読むだけ・非侵襲）。
        assert_eq!(a.net_state_hash_hex(), h);

        // クランプ: 下限0（負は0に）・上限8（それ以上は8に）。
        a.set_w_elev(-5.0);
        assert_eq!(a.params.w_elev, 0.0);
        a.set_w_elev(9999.0);
        assert_eq!(a.params.w_elev, 8.0);

        // 非侵襲・決定性: 同一 seed・同一操作列・同一 w_elev → 同一 net_state_hash。
        let mut x = NetSim::new_net(7);
        let mut y = NetSim::new_net(7);
        x.set_w_elev(4.0);
        y.set_w_elev(4.0);
        for _ in 0..60 {
            x.step();
            y.step();
        }
        assert_eq!(x.net_state_hash_hex(), y.net_state_hash_hex());
    }
}
