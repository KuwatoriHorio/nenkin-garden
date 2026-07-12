# タスク: netphys-002 — 網 Physarum Stage 2（前進波移動＋Tero 効率改善）

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [netphys-001](task-netphys-001.md)（Stage 1・①②⑤⑥）完了・全テスト緑。設計メモ
[design-memo-netphys-000.md](design-memo-netphys-000.md)。

> **状態: 🔵 未着手（§8.1 で人間確認待ち）**

---

## 背景
netphys-001（Stage 1）で「扇状探索→衝突で網化→周期 consolidation（Kirchhoff＋Tero 刈り込み・最外周端子）」
が動き、餌を結ぶ連結網を保存則・決定性・有界のもとで作れた。Phase 4 の"歩く"（前進波移動）は最小限のみで
合否に問わなかった。本タスクは **Stage 2 ＝ ③前進波で有意に移動・④consolidation で効率改善** を達成する。

## task_id
`netphys-002`

## goal
network Physarum が **観測できる前進波として外へ移動**し（後方を刈って前線へ原形質を送り重心が動く）、
かつ **consolidation が端子間の輸送効率を改善**する（背骨へ刈り込み）ことを headless で実証する。
netphys-001 の ①②⑤⑥（網化・餌連結・保存/決定/境界/忌避・有界）は**引き続き緑を保つ**。
**新モデル内の力学調整/拡張**（Jones/tree とは無関係・独立）。

## 実装方針（設計メモ Phase 3〜4）
- **前進波を効かせる**（`src/netphys/step.rs` の consolidation/relocation・NetParams 調整）:
  - consolidation で選んだ**最外周ノードを次の前線**にし、**後方（背骨に載らない内側枝）を確実に prune**して、
    刈った `Σ D·L` を free_budget へ戻す → 前線の再拡散に回す（原形質の前送り＝translocation）。
  - これで**重心/最外周が外へ動く**。その場脈動でなく**正味の外向き変位**が出るよう、最外周選定を外向きに・
    後方刈りを効かせる。NetParams（周期 N・k・Tero 係数・prune ε・探索率 等）を実測で調整。
- **効率改善**を測れる形にする: consolidation の前後で端子間の総コンダクタンス／`transport_efficiency`（HHI）を
  比較し、後 ≥ 前（背骨集約で改善）。既存 Kirchhoff（`netphys_kirchhoff_solve`）を利用。
- ①②⑤⑥ を壊さない範囲で調整する（これらは性質テストで、パラメータ調整でも真であり続けるはず。もし
  移動を効かせると餌連結②や網化①が壊れるなら、それは設計の綱引きなので §8 で停止・相談）。

## acceptance_test（Stage 2・headless・複数シード中央値・S9 部分集合 [1,42,1337]）
3. **前進波で移動**: K 回の consolidation（K·N tick）を回すと、コロニーの**重心（または最外周半径）が初期から
   有意に外へ移動**する（中央値でしきい超）。その場で脈動するだけ（正味変位ゼロ）ではないこと。
4. **効率化（Tero）**: consolidation の**前後**で端子間の総コンダクタンス／`transport_efficiency` が**改善**する
   （中央値で 後 ≥ 前・有意差）。または K サイクルで効率が上昇トレンド。
- **回帰**: `tests/netphys_001.rs` の①②⑤⑥が**緑のまま**（性質は保つ）。Jones/tree/analysis 全テストも緑。
- パラメータは**探索用一時テスト**で実測 → 決定後に削除。まず赤を確認してから実装。新規テストは `tests/netphys_002.rs`。

## constraints
- **編集可**: `src/netphys/{state,step}.rs`（consolidation/relocation の調整・NetParams 既定/追加）、
  新規 `tests/netphys_002.rs`、`src/bin/run_netphys.rs`（任意）。
- **編集不可（人間所有・§7/§11）**: Jones/tree の不変条件・ゴールデン・受け入れ、`tests/netphys_001.rs` の
  **アサート**（①②⑤⑥ は性質として保つ・弱めない）、`test_harness_001.rs`/`metric_thresholds_001.rs` の baseline、
  analysis 受け入れ、規約・設計軸（§0）。**Jones/tree のモデルコードは変更しない**。
- **決定性**: 探索は単一 PRNG・id 昇順。Kirchhoff/刈り込みは決定的。**保存則を壊さない**（前送りの会計をテストで締める）。
- **過学習禁止**: 単一シードだけで移動/改善する調整は不可。複数シード中央値。§7: 通すために tick/seed/cap を都合よく縮小しない。

## seeds
`[1, 42, 1337]`（可能なら S9 中央値）

## §8 停止の指針（このタスク特有・重要）
**前進波の安定移動が本タスク最大の難所**。次に当たったら**§7 のごまかしをせず停止**し、成立した部分と詰まり・
選択肢を報告する（部分成立で構わない）:
- その場脈動から抜けられない（正味の外向き変位が出ない）。
- 移動を効かせると①網化/②餌連結/⑤保存則が壊れる（設計の綱引き）。
- 爆発 vs 枯渇の甘い帯が無く、③と⑥（有界）が両立しない。
無理に③④を通すために cap/tick/解像度を縮小したり、netphys-001 の①②⑤⑥を弱めたりしない。

## このタスクでやらないこと
- 描画（前進波が動くデモ反映は後続の render-net タスク）。今回はヘッドレス＋テストのみ。
- Jones/tree の変更。β（ノード貯留担体）への切替（担体A のまま）。

## 関連
- モデル: [src/netphys/step.rs](../src/netphys/step.rs)（consolidation/relocation・`netphys_kirchhoff_solve`）、
  [netphys-001](task-netphys-001.md)、[設計メモ](design-memo-netphys-000.md)。
