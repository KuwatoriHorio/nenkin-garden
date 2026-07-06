//! 1ステップの遷移 step（設計メモ §5）。決定性契約（§4）を厳守。
//!
//! - 乱数は state.rng の単一 PRNG のみ。
//! - エージェント更新は index 昇順、固定3本/体（tie / 前進確率 / 失敗時方位）。
//! - 感知は tick 開始時スナップショット trail_read から読む。定着は別バッファ
//!   trail_write に書き、拡散・減衰後に trail を差し替える
//!   → 同一 tick 内の処理順がセンシング結果に影響しない。

use crate::params::Params;
use crate::state::{apply_op, Op, State};
use crate::world::World;

const TWO_PI: f64 = std::f64::consts::TAU;

#[inline]
fn habitability(e: f64, p: &Params) -> f64 {
    // E_hi 超で smoothstep により 0 へ（定着差=忌避③）
    let denom = (p.habit_top - p.e_hi).max(1e-9);
    let t = ((e - p.e_hi) / denom).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    1.0 - smooth
}

#[inline]
fn decay(e: f64, p: &Params) -> f64 {
    // 高標高ほど大きい（減衰差=忌避④）
    let d = p.decay_base + (p.decay_high - p.decay_base) * e.clamp(0.0, 1.0);
    d.clamp(0.0, 0.999)
}

/// 最近傍セルで trail/E をサンプル。範囲外・海は sea=true。
/// 返り値: (trail_val, e_val, sea, cix, ciy)
#[inline]
fn sample(px: f64, py: f64, trail: &[f32], world: &World) -> (f64, f64, bool, usize, usize) {
    let (h, w) = (world.h, world.w);
    let fx = px.floor();
    let fy = py.floor();
    let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
    let cix = (fx.max(0.0) as usize).min(w - 1);
    let ciy = (fy.max(0.0) as usize).min(h - 1);
    let idx = ciy * w + cix;
    let land = world.land_mask[idx] && inb;
    (trail[idx] as f64, world.e[idx] as f64, !land, cix, ciy)
}

/// 3x3 平均ブラー（拡散）。境界外は0（海）。
fn blur3(src: &[f64], h: usize, w: usize) -> Vec<f64> {
    let mut out = vec![0.0f64; h * w];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let ny = y as i64 + dy;
                    let nx = x as i64 + dx;
                    if ny >= 0 && ny < h as i64 && nx >= 0 && nx < w as i64 {
                        s += src[ny as usize * w + nx as usize];
                    }
                }
            }
            out[y * w + x] = s / 9.0;
        }
    }
    out
}

/// state を 1tick 進める（in-place）。
pub fn step(state: &mut State, world: &World, p: &Params, ops: &[Op]) {
    let (h, w) = (world.h, world.w);

    // この tick の op を適用
    for op in ops {
        apply_op(state, op);
    }

    // tick開始スナップショット（不変）と書き込みバッファ
    let trail_read: Vec<f32> = state.trail.clone();
    let mut trail_write = vec![0.0f64; h * w];

    // --- エージェント更新（index昇順・固定3本/体）---
    let n = state.n_agents();
    for i in 0..n {
        let r_tie = state.rng.next_f64();
        let r_move = state.rng.next_f64();
        let r_head = state.rng.next_f64();

        let x = state.ax[i] as f64;
        let y = state.ay[i] as f64;
        let heading = state.ah[i] as f64;

        // 1. 感知（前方3点）
        let sense = |angle_off: f64| -> f64 {
            let hx = heading + angle_off;
            let sx = x + p.sensor_dist * hx.cos();
            let sy = y + p.sensor_dist * hx.sin();
            let (tv, ev, sea, _, _) = sample(sx, sy, &trail_read, world);
            if sea {
                -p.sea_penalty
            } else {
                tv - p.w_e * ev
            }
        };
        let left = sense(-p.sensor_angle);
        let center = sense(0.0);
        let right = sense(p.sensor_angle);

        // 2. 旋回（Jones則）
        let straight = center >= left && center >= right;
        let mut delta = 0.0;
        if !straight {
            if left > right {
                delta = -p.turn_speed;
            } else if right > left {
                delta = p.turn_speed;
            } else {
                // 均衡 → ランダム側
                delta = if r_tie < 0.5 { -p.turn_speed } else { p.turn_speed };
            }
        }
        let mut new_heading = heading + delta;

        // 3. 前進（傾斜コスト）
        let tx = x + p.step_size * new_heading.cos();
        let ty = y + p.step_size * new_heading.sin();
        let (_, e_target, sea_target, _, _) = sample(tx, ty, &trail_read, world);
        let (_, e_cur, _, _, _) = sample(x, y, &trail_read, world);
        let d_e = (e_target - e_cur).max(0.0);
        let p_move = (-p.k_slope * d_e).exp();
        let move_ok = (r_move < p_move) && !sea_target;

        let (nx, ny) = if move_ok { (tx, ty) } else { (x, y) };
        if !move_ok {
            // 前進失敗 → ランダムに新方位
            new_heading = r_head * TWO_PI;
        }

        // 4. 定着（現在セルへ, H(E)重み）
        let (_, e_here, _, cix, ciy) = sample(nx, ny, &trail_read, world);
        let amount = p.deposit * habitability(e_here, p);
        trail_write[ciy * w + cix] += amount;

        // 書き戻し（正準順序=index順を維持）
        state.ax[i] = nx as f32;
        state.ay[i] = ny as f32;
        state.ah[i] = new_heading.rem_euclid(TWO_PI) as f32;
    }

    // --- 5. 砂糖回収（id昇順で決定的に）---
    let m = state.sugar_id.len();
    if m > 0 && n > 0 {
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by_key(|&i| state.sugar_id[i]);
        let r2 = p.sugar_radius * p.sugar_radius;
        for &j in &order {
            if state.sugar_remaining[j] <= 0.0 {
                continue;
            }
            let sx = state.sugar_x[j];
            let sy = state.sugar_y[j];
            let mut reached = false;
            for k in 0..state.ax.len() {
                let dx = state.ax[k] as f64 - sx;
                let dy = state.ay[k] as f64 - sy;
                if dx * dx + dy * dy <= r2 {
                    reached = true;
                    break;
                }
            }
            if reached {
                let gain = p.collect_rate.min(state.sugar_remaining[j]);
                state.biomass += gain;
                state.collected_total += gain;
                state.sugar_remaining[j] -= gain;
            }
        }
    }

    // --- 砂糖ビーコン（残量ありの source が毎tick trailに強い誘引を加算）---
    for j in 0..m {
        if state.sugar_remaining[j] > 0.0 {
            let fx = state.sugar_x[j].floor();
            let fy = state.sugar_y[j].floor();
            if fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64 {
                let cix = fx as usize;
                let ciy = fy as usize;
                trail_write[ciy * w + cix] += p.sugar_beacon;
            }
        }
    }

    // --- 6. 成長（スポーン）---
    let ncur = state.n_agents();
    let max_agents = p
        .agent_cap_max
        .min((p.agent_cap_base + p.agent_cap_slope * state.biomass) as usize);
    let want = if max_agents > ncur {
        p.spawn_per_tick.min(max_agents - ncur)
    } else {
        0
    };
    let affordable = if p.spawn_cost > 0.0 {
        (state.biomass / p.spawn_cost) as usize
    } else {
        want
    };
    let n_spawn = want.min(affordable);
    if n_spawn > 0 && ncur > 0 {
        // 決定的な描画順: parent → jx → jy → heading の順で乱数消費
        let pidx: Vec<usize> = (0..n_spawn)
            .map(|_| state.rng.gen_range(ncur as u64) as usize)
            .collect();
        let jx: Vec<f64> = (0..n_spawn)
            .map(|_| (state.rng.next_f64() * 2.0 - 1.0) * p.spawn_jitter)
            .collect();
        let jy: Vec<f64> = (0..n_spawn)
            .map(|_| (state.rng.next_f64() * 2.0 - 1.0) * p.spawn_jitter)
            .collect();
        let nh: Vec<f64> = (0..n_spawn).map(|_| state.rng.next_f64() * TWO_PI).collect();

        for s in 0..n_spawn {
            let par = pidx[s];
            let px_ = state.ax[par] as f64 + jx[s];
            let py_ = state.ay[par] as f64 + jy[s];
            // スポーン先が陸でなければ親位置にフォールバック（境界不変条件を守る）
            let inb = px_ >= 0.0 && px_ < w as f64 && py_ >= 0.0 && py_ < h as f64;
            let onland = if inb {
                world.land_mask[(py_.floor() as usize) * w + (px_.floor() as usize)]
            } else {
                false
            };
            let (fx, fy) = if onland {
                (px_, py_)
            } else {
                (state.ax[par] as f64, state.ay[par] as f64)
            };
            state.ax.push(fx as f32);
            state.ay.push(fy as f32);
            state.ah.push(nh[s] as f32);
        }

        let cost = n_spawn as f64 * p.spawn_cost;
        state.biomass -= cost;
        state.consumed_total += cost;
    }

    // --- 7. 拡散・減衰 ---
    let blurred = blur3(&trail_write, h, w);
    for i in 0..h * w {
        let mixed = (1.0 - p.diffuse_rate) * trail_write[i] + p.diffuse_rate * blurred[i];
        let e = world.e[i] as f64;
        let mut v = mixed * (1.0 - decay(e, p));
        if !world.land_mask[i] {
            v = 0.0; // 海はマスク
        }
        state.trail[i] = v as f32;
    }

    state.tick += 1;
}
