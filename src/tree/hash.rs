//! tree_state_hash（決定性検証用の正準ハッシュ）。既存 src/hash.rs の方式（FNV-1a・量子化）を
//! 参考にした新規実装。現行 hash.rs は変更しない。

use super::state::{path_len, TreeParams, TreeState};

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

/// 64bit の決定論ハッシュ。同一 TreeState（量子化後）→同一値。正準順序: ノードindex昇順・
/// 砂糖id昇順・rng内部状態。
pub fn tree_state_hash(s: &TreeState, p: &TreeParams) -> u64 {
    let mut h = FNV64_OFFSET;

    h = feed_u64(h, s.tick);
    h = feed_i64(h, quant(s.b_free, p.q_vol));

    // ノード（index順=正準）: parent(-1=None) + 量子化座標 + 量子化パス距離
    h = feed_u64(h, s.nodes.len() as u64);
    for i in 0..s.nodes.len() {
        let par: i64 = match s.nodes[i].parent {
            Some(pi) => pi as i64,
            None => -1,
        };
        h = feed_i64(h, par);
        h = feed_i64(h, quant(s.nodes[i].x as f64, p.q_pos));
        h = feed_i64(h, quant(s.nodes[i].y as f64, p.q_pos));
        h = feed_i64(h, quant(path_len(s, i), p.q_vol));
    }

    // 帳簿
    h = feed_i64(h, quant(s.collected_total, p.q_vol));
    h = feed_i64(h, quant(s.consumed_total, p.q_vol));

    // 砂糖源（id 昇順に正準化）
    let mut order: Vec<usize> = (0..s.sugar_id.len()).collect();
    order.sort_by_key(|&i| s.sugar_id[i]);
    h = feed_u64(h, order.len() as u64);
    for &i in &order {
        h = feed_u64(h, s.sugar_id[i]);
        h = feed_i64(h, quant(s.sugar_x[i], p.q_pos));
        h = feed_i64(h, quant(s.sugar_y[i], p.q_pos));
        h = feed_i64(h, quant(s.sugar_strength[i], p.q_vol));
        h = feed_i64(h, quant(s.sugar_remaining[i], p.q_vol));
    }

    // rng 内部状態
    for word in s.rng.state() {
        h = feed_u64(h, word);
    }

    h
}
