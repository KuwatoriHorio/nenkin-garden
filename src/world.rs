//! 場（World）: 陸海マスクと標高場 E（設計メモ §3-場, §2）。
//! 読み取り専用・不変。シミュレーション seed に依存しない（全 seed 共通の世界）。
//! ここでは決定論的に合成した「仮想列島」を生成する（実地形は別タスク）。

use crate::params::Params;

#[derive(Clone, Debug)]
pub struct World {
    pub h: usize,
    pub w: usize,
    pub land_mask: Vec<bool>, // [h*w] true=陸
    pub e: Vec<f32>,          // [h*w] 0=海面〜1=最高峰（海は0）
}

impl World {
    #[inline]
    pub fn idx(&self, y: usize, x: usize) -> usize {
        y * self.w + x
    }

    /// core-002: 既定ホーム座標を決定論的に選ぶ（seed 非依存）。
    /// 主要島（最大の低標高 E<e_lo 連結塊）の重心に最も近い低標高陸セルの中心。
    /// 低標高陸が無ければ任意の陸セルの重心へフォールバック。
    pub fn default_home(&self, e_lo: f64) -> (f64, f64) {
        let (h, w) = (self.h, self.w);
        // 低標高帯（land && E<e_lo）の重心
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut cnt = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if self.land_mask[i] && (self.e[i] as f64) < e_lo {
                    sx += x as f64;
                    sy += y as f64;
                    cnt += 1;
                }
            }
        }
        if cnt == 0 {
            // 低標高帯が空 → 全陸の重心
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if self.land_mask[i] {
                        sx += x as f64;
                        sy += y as f64;
                        cnt += 1;
                    }
                }
            }
        }
        assert!(cnt > 0, "world has no land cells");
        let (gx, gy) = (sx / cnt as f64, sy / cnt as f64);
        // 重心に最も近い「低標高陸セル（無ければ陸セル）」の中心へスナップ（決定的）
        let mut best = usize::MAX;
        let mut best_d = f64::INFINITY;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let ok = self.land_mask[i] && ((self.e[i] as f64) < e_lo || cnt == 0);
                if !ok {
                    continue;
                }
                let d = (x as f64 - gx).powi(2) + (y as f64 - gy).powi(2);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
        }
        // 低標高帯が空でスナップ候補が無い場合に備え、陸セルでも再走査
        if best == usize::MAX {
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if !self.land_mask[i] {
                        continue;
                    }
                    let d = (x as f64 - gx).powi(2) + (y as f64 - gy).powi(2);
                    if d < best_d {
                        best_d = d;
                        best = i;
                    }
                }
            }
        }
        let cx = (best % w) as f64 + 0.5;
        let cy = (best / w) as f64 + 0.5;
        (cx, cy)
    }
}

fn gaussian_bump(h: usize, w: usize, cy: f64, cx: f64, sy: f64, sx: f64, out: &mut [f64], amp: f64) {
    for y in 0..h {
        for x in 0..w {
            let dy = y as f64 - cy;
            let dx = x as f64 - cx;
            let v = (-((dy * dy) / (2.0 * sy * sy) + (dx * dx) / (2.0 * sx * sx))).exp();
            out[y * w + x] += amp * v;
        }
    }
}

/// 複数の島 + 山脈状の標高場を手続き生成（設計メモ §12）。seed 非依存で完全に決定的。
pub fn make_synthetic_archipelago(p: &Params) -> World {
    let (h, w) = (p.h, p.w);
    let mut height = vec![0.0f64; h * w];

    // (cy比, cx比, sy比, sx比, 山の高さ)
    let islands = [
        (0.30, 0.28, 0.14, 0.16, 1.00),
        (0.62, 0.40, 0.11, 0.13, 0.85),
        (0.45, 0.68, 0.13, 0.11, 0.95),
        (0.75, 0.72, 0.09, 0.10, 0.70),
        (0.20, 0.60, 0.08, 0.09, 0.60),
    ];
    for (cyr, cxr, syr, sxr, amp) in islands {
        gaussian_bump(
            h,
            w,
            cyr * h as f64,
            cxr * w as f64,
            syr * h as f64,
            sxr * w as f64,
            &mut height,
            amp,
        );
    }

    // 低周波の起伏（決定的な正弦波）で海岸線を非自明にする
    let two_pi = std::f64::consts::TAU;
    for y in 0..h {
        for x in 0..w {
            let ripple = 0.06
                * (two_pi * x as f64 / w as f64 * 3.0).sin()
                * (two_pi * y as f64 / h as f64 * 2.0).cos();
            height[y * w + x] += ripple;
        }
    }

    // 海面しきいで陸海を決める
    let sea_level = 0.18;
    let land_mask: Vec<bool> = height.iter().map(|&v| v > sea_level).collect();

    // E: 陸上のみ [0,1] に正規化。海は0。
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for i in 0..h * w {
        if land_mask[i] {
            lo = lo.min(height[i]);
            hi = hi.max(height[i]);
        }
    }
    let span = (hi - lo).max(1e-9);
    let e: Vec<f32> = (0..h * w)
        .map(|i| {
            if land_mask[i] {
                (((height[i] - lo) / span).clamp(0.0, 1.0)) as f32
            } else {
                0.0
            }
        })
        .collect();

    World { h, w, land_mask, e }
}
