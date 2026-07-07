//! render-001: 粘菌ガーデン ブラウザ対話デモ（WASM, render レイヤ）。
//!
//! 決定論コア（`nenkin_garden`）を wasm で実行し、trail 網を RGBA バッファへ描画する。
//! render は **State を読むだけ**（core ← render の一方向依存, 設計メモ §2）。
//! プレイヤーの動詞は §0 の「砂糖を置く／取り除く」「時間速度の変更」のみ。
//! 砂糖 op は tick 境界で適用し、決定性契約（§2/§4）を保つ。

use wasm_bindgen::prelude::*;

use nenkin_garden::analysis::analyze;
use nenkin_garden::graph_svg::mst_edge_set;
use nenkin_garden::params::Params;
use nenkin_garden::state::{apply_op, initial_state, Op, State};
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
        let params = Params::default();
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

    /// 現在 State を RGBA バッファへ描画する（State は読むだけ・非侵襲）。
    pub fn render(&mut self) {
        let (w, h) = (self.world.w, self.world.h);
        let maxt = self.state.trail.iter().cloned().fold(0.0f32, f32::max).max(1e-6) as f64;
        let glow = (124.0, 246.0, 152.0);
        for i in 0..w * h {
            let (r, g, b) = if self.world.land_mask[i] {
                let e = self.world.e[i] as f64;
                let base = land_color(e);
                let t = (self.state.trail[i] as f64 / maxt).clamp(0.0, 1.0);
                let a = (t * 1.6).min(1.0);
                lerp(base, glow, a)
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
        a.render();
        assert_eq!(a.state_hash_hex(), h);
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
}
