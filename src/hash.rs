//! state_hash（設計メモ §9）: 決定性の要。
//! 順序を固定して正準シリアライズし 64bit FNV-1a を取る。量子化で浮動小数ノイズ耐性。

use crate::params::Params;
use crate::state::State;

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

#[inline]
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV64_PRIME);
    }
    h
}

#[inline]
fn quant(x: f64, q: f64) -> i64 {
    (x / q).round() as i64
}

#[inline]
fn feed_i64(h: u64, v: i64) -> u64 {
    fnv1a(h, &v.to_le_bytes())
}

#[inline]
fn feed_u64(h: u64, v: u64) -> u64 {
    fnv1a(h, &v.to_le_bytes())
}

fn feed_quant_slice_f32(mut h: u64, a: &[f32], q: f64) -> u64 {
    for &v in a {
        h = feed_i64(h, quant(v as f64, q));
    }
    h
}

fn feed_quant_slice_f64(mut h: u64, a: &[f64], q: f64) -> u64 {
    for &v in a {
        h = feed_i64(h, quant(v, q));
    }
    h
}

/// 64bit の決定論ハッシュ。同一 State（量子化後）→ 同一値。正準順序は §9。
pub fn state_hash(s: &State, p: &Params) -> u64 {
    let mut h = FNV64_OFFSET;

    // 1. tick
    h = feed_u64(h, s.tick);

    // 2. biomass
    h = feed_i64(h, quant(s.biomass, p.q_bio));

    // 3. agents（index順が正準）
    h = feed_u64(h, s.n_agents() as u64);
    h = feed_quant_slice_f32(h, &s.ax, p.q_pos);
    h = feed_quant_slice_f32(h, &s.ay, p.q_pos);
    h = feed_quant_slice_f32(h, &s.ah, p.q_pos);

    // 4. trail（行優先）
    h = feed_quant_slice_f32(h, &s.trail, p.q_trail);

    // 5. 帳簿
    h = feed_i64(h, quant(s.collected_total, p.q_bio));
    h = feed_i64(h, quant(s.consumed_total, p.q_bio));

    // 6. sugar 源（id 昇順に正準化してから連結）
    let mut order: Vec<usize> = (0..s.sugar_id.len()).collect();
    order.sort_by_key(|&i| s.sugar_id[i]);
    h = feed_u64(h, order.len() as u64);
    for &i in &order {
        h = feed_u64(h, s.sugar_id[i]);
    }
    let sx: Vec<f64> = order.iter().map(|&i| s.sugar_x[i]).collect();
    let sy: Vec<f64> = order.iter().map(|&i| s.sugar_y[i]).collect();
    let sst: Vec<f64> = order.iter().map(|&i| s.sugar_strength[i]).collect();
    let srem: Vec<f64> = order.iter().map(|&i| s.sugar_remaining[i]).collect();
    h = feed_quant_slice_f64(h, &sx, p.q_pos);
    h = feed_quant_slice_f64(h, &sy, p.q_pos);
    h = feed_quant_slice_f64(h, &sst, p.q_bio);
    h = feed_quant_slice_f64(h, &srem, p.q_bio);

    // 7. rng 内部状態
    for word in s.rng.state() {
        h = feed_u64(h, word);
    }

    h
}
