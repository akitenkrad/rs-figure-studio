# Figurine Studio -- データフロー

## スプライトパイプライン全体図

MagicaVoxel 等のボクセルエディタからレンダリングした画像，または生成 AI で作成したキャラクター画像を入力とし，最終的に Bevy Engine で利用可能なスプライトシート + メタデータ JSON を出力する．AI 生成パスと従来インポートパスは共存し，いずれも共通パイプライン（BG Removal 以降）に合流する．

```
┌────────────────────────────────── AI 生成パス ──────────────────────────────┐
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐  ┌──────────────┐       │
│  │Concept Art   │→ │Pixel Art     │→ │ Direction│→ │ Animation    │──┐    │
│  │(1枚:txt2img) │  │(変換:img2img)│  │ (4方向)  │  │ (フレーム群) │  │    │
│  └──────────────┘  └──────────────┘  └──────────┘  └──────────────┘  │    │
│     (A1a)              (A1b)            (A2)            (A3)          │    │
└──────────────────────────────────────────────────────────────────────│────┘
                                                       │
┌──────────────── 従来インポートパス ──────────────────│──────────────────────┐
│  ┌────────┐    ┌────────────┐                        │                      │
│  │ Import │ →  │ AI Texture │ ───────────────────────┤                      │
│  │        │    │ (ComfyUI)  │                        │                      │
│  │ raw    │    │ ai_        │                        │                      │
│  │        │    │ processed  │                        │                      │
│  └────────┘    └────────────┘                        │                      │
│   (1)              (2)                               │                      │
└──────────────────────────────────────────────────────│──────────────────────┘
                                                       │
               ┌───────────────────────────────────────┘
               ▼
┌────────────────────── 共通パイプライン ──────────────────────────────────────┐
│  ┌────────────┐    ┌───────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ BG Removal │ →  │ Normalize │ →  │ Spritesheet │ →  │ Bevy Export │     │
│  │ (ONNX)     │    │           │    │             │    │             │     │
│  │ bg_removed │    │ finalized │    │ spritesheet │    │ characters/ │     │
│  └────────────┘    └───────────┘    └─────────────┘    └─────────────┘     │
│      (3)              (4)               (5)               (6)               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### ステータス遷移

```
                     ┌── AI生成パス ──┐
                     │                │
                     ▼                │
generated ──→ raw ──→ ai_processed ──→ bg_removed ──→ finalized
                │         │                │
                │         │                └──→ finalized (Normalize 直接適用可)
                │         │
                │         └──→ finalized (BG Removal スキップ時)
                │
                └──→ finalized (AI Texture + BG Removal スキップ時)
```

スプライトの `status` フィールドは `sprites` テーブルで管理され，各ステージの完了時に更新される．ステージのスキップ（例: AI テクスチャを適用せず直接背景除去）も可能である．

`generated` は AI 生成パスで作成されたスプライトの初期ステータスである．AI 生成の結果をユーザーが確認・承認した時点でステータスが `raw` に遷移し，以降は従来インポートパスと同一の共通パイプラインに入る．これにより，AI 生成スプライトと手動インポートスプライトが同じ後処理フローを共有できる．

**参照ファイル:** `src-tauri/src/models/sprite.rs`（`SpriteStatus` 列挙型）

---

## 各ステージの処理詳細

### (A1a) Concept Art -- コンセプトアート生成

テキストプロンプトから詳細なキャラクターイラスト（コンセプトアート）を1枚生成するステージ．

**生成バックエンド:**

- **ComfyUI（ローカル）:** `concept_art_generation.json` テンプレートによる txt2img
- **クラウド API:** PixelLab / fal.ai を使用したリモート生成

**出力先:** `{base_path}/{character_name}/concepts/`

**処理フロー:**

1. ユーザーがテキストプロンプト・スタイル設定を入力
2. 生成バックエンド（ComfyUI またはクラウド API）にリクエストを送信
3. 生成結果を `concepts/` ディレクトリに保存
4. ユーザーが結果を確認・選別・リトライ可能
5. 承認されたコンセプトアートが (A1b) Pixel Art Conversion の入力となる

**参照ファイル:** `docs/architecture/ai-generation.md`

---

### (A1b) Pixel Art Conversion -- ピクセルアート変換

コンセプトアート画像をピクセルアートに変換するステージ（img2img）．

**生成バックエンド:**

- **ComfyUI（ローカル）:** `pixel_art_conversion.json` テンプレートによる img2img 変換
- **クラウド API:** クラウド API 側でのピクセルアート変換

**出力先:** `{base_path}/{character_name}/concepts/`

**処理フロー:**

1. (A1a) で承認されたコンセプトアートを入力画像として使用
2. img2img 変換でピクセルアートに変換
3. `pixel_grid_width` / `pixel_grid_height` が設定されている場合，`pixelate_image()` による Rust 側グリッド整列を自動適用
4. 変換結果が (A2) Direction Expansion の入力となる

**参照ファイル:** `src-tauri/src/commands/image_processing.rs`（`pixelate_image` 関数）

---

### (A2) Direction Expansion -- 方向展開

コンセプト画像から4方向（down，left，right，up）の基本ポーズを生成するステージ．

**生成バックエンド:**

- **ComfyUI（ローカル）:** IP-Adapter（コンセプト画を参照）+ ControlNet（方向別 OpenPose スケルトン）
- **クラウド API:** PixelLab ワンクリック回転 / fal.ai マルチリファレンス

**出力先:** `{base_path}/{character_name}/generated/`

**一貫性確保の仕組み:**

- Seed 固定 + 同一 IP-Adapter 参照画像の使用により，方向間でキャラクターの外見一貫性を維持
- ControlNet の方向別 OpenPose スケルトンで正しいポーズ方向を強制

---

### (A3) Animation Expansion -- アニメーション展開

各方向の基本ポーズからアニメーションフレームを生成するステージ．

**生成バックエンド:**

- **ComfyUI（ローカル）:** IP-Adapter + ControlNet，フレーム単位バッチ生成（seed 固定）
- **クラウド API:** Retro Diffusion アニメーションプリセット

**出力先:** `{base_path}/{character_name}/generated/`

**処理フロー:**

1. 各方向の基本ポーズ画像を参照画像として使用
2. アニメーション種別（idle，walk，attack 等）ごとにフレームを生成
3. 生成完了後，`sprites` テーブルに `generated` ステータスで登録
4. ユーザーが生成結果を確認・承認すると `raw` ステータスに遷移し，共通パイプラインに入る

**キーフレーム補間（Sprint 8 追加）:**

`AnimationParams` に `keyframes` および `interpolation` フィールドが追加された．キーフレームが指定された場合，指定されたフレームのみ AI で生成し，中間フレームは補間アルゴリズムで自動生成される．これにより AI 生成回数を削減しつつ滑らかなアニメーションを実現する．

| 補間方式 | 説明 |
|---------|------|
| crossfade | 前後のキーフレームをクロスフェードで補間 |
| nearest | 最も近いキーフレームのコピーで補間 |

**参照ファイル:** `src-tauri/src/services/frame_interpolation.rs`

---

### (1) Import -- スプライトインポート

**コマンド:** `import_sprites`
**ソース:** `src-tauri/src/commands/character_commands.rs`

ユーザーがファイル選択ダイアログで画像ファイルを選択すると，以下の処理が実行される:

1. ファイル名を `{character}_{direction}_{animation}_{frame}.png` 形式でパース
2. 画像を `{base_path}/{character_name}/raw/` にコピー
3. DB の `sprites` テーブルにレコードを一括作成（`create_sprites_batch`）
4. スプライトの初期ステータスは `raw`

**ファイル名パース規則:**

`FilenameParser`（`src-tauri/src/services/filename_parser.rs`）が以下のバリエーションに対応する:

| パターン | 例 |
|---------|-----|
| 標準 | `warrior_down_idle_00.png` |
| ハイフン区切り | `warrior-down-idle-00.png` |
| 大文字混在 | `Warrior_Down_Idle_00.png` |
| ゼロ埋めなし | `warrior_down_walk_1.png` |
| フレーム番号なし | `warrior_down_idle.png`（フレーム 0 として扱う） |
| 複合キャラクター名 | `dark_knight_left_walk_03.png` |

対応拡張子: `.png`, `.jpg`, `.jpeg`, `.webp`, `.bmp`, `.tiff`, `.tif`

**ディレクトリ構造（キャラクター作成時）:**

キャラクター作成時に以下のサブディレクトリが自動生成される:

```
{base_path}/{character_name}/
├── concepts/             # (A1a-A1b) コンセプトアート・ピクセルアート画像
│   ├── concept_001.png
│   └── concept_002.png
├── generated/            # (A2-A3) AI生成スプライト
│   ├── warrior_down_idle_00.png
│   └── ...
├── raw/                  # (1) インポート元画像
│   ├── warrior_down_idle_00.png
│   └── ...
├── ai_processed/         # (2) ComfyUI 質感変換後
├── bg_removed/           # (3) 背景除去後
├── normalized/           # (4) 正規化後
└── spritesheet/          # (5) スプライトシート
```

---

### (2) AI Texture -- ComfyUI 質感変換

**コマンド:** `process_batch_comfyui`
**ソース:** `src-tauri/src/commands/comfyui.rs`

ComfyUI サーバーと連携し，ボクセルレンダリング画像にフィギュア風の質感を付与する．

**処理フロー（1スプライトあたり）:**

```
1. 入力画像を ComfyUI にアップロード
   POST /upload/image (multipart/form-data)
        ↓
2. ワークフロー JSON のプレースホルダを実値に置換
        ↓
3. ワークフローをキューに投入
   POST /prompt
        ↓
4. ポーリングで完了待ち（1秒間隔，120秒タイムアウト）
   GET /history/{prompt_id}
        ↓
5. 結果画像をダウンロード・保存
   GET /view?filename={}&subfolder={}&type=output
        ↓
6. DB 更新: processed_path 設定，status → ai_processed
```

**入力画像の優先順位:** `processed_path` > `raw_path`

**出力先:** `{base_path}/{character_name}/ai_processed/`

**エラーハンドリング:**
- 個別のスプライト処理失敗はスキップし，バッチ全体を継続
- `comfyui-progress` イベントで進捗を通知（`current`, `total`, `status`, `current_file`）
- HTTP リクエストは最大3回リトライ（Exponential Backoff: 1s, 2s, 4s）

**ComfyUI クライアント API:**

| メソッド | HTTP | エンドポイント | 用途 |
|---------|------|--------------|------|
| `health_check` | GET | `/system_stats` | 接続確認（5秒タイムアウト） |
| `upload_image` | POST | `/upload/image` | 画像アップロード |
| `queue_prompt` | POST | `/prompt` | ワークフロー実行キュー投入 |
| `get_history` | GET | `/history/{prompt_id}` | 実行結果取得 |
| `get_image` | GET | `/view?filename=...` | 結果画像ダウンロード |
| `wait_for_completion` | GET (polling) | `/history/{prompt_id}` | 完了待ち |

**参照ファイル:** `src-tauri/src/services/comfyui_client.rs`

---

### ComfyUI ワークフロープレースホルダ仕様

ワークフロー JSON 内のプレースホルダは `{{key}}` 形式で記述し，処理時に実値に置換される．

**対応プレースホルダ（17種）:**

| プレースホルダ | 型 | 説明 | 置換元 |
|--------------|-----|------|-------|
| `{{input_image}}` | 文字列 | アップロード後のファイル名 | ComfyUI アップロード結果 |
| `{{positive_prompt}}` | 文字列 | ポジティブプロンプト | `ProcessingParams.positive_prompt` |
| `{{negative_prompt}}` | 文字列 | ネガティブプロンプト | `ProcessingParams.negative_prompt` |
| `{{seed}}` | 数値 | 乱数シード | `ProcessingParams.seed` またはランダム |
| `{{steps}}` | 数値 | サンプリングステップ数 | `ProcessingParams.steps` |
| `{{cfg_scale}}` | 数値 | CFG スケール | `ProcessingParams.cfg_scale` |
| `{{denoise_strength}}` | 数値 | デノイズ強度 | `ProcessingParams.denoise_strength` |
| `{{controlnet_weight}}` | 数値 | ControlNet ウェイト | `ProcessingParams.controlnet_weight` |
| `{{checkpoint_name}}` | 文字列 | チェックポイントモデル名 | 動的モデル選択 |
| `{{lora_name}}` | 文字列 | LoRA モデル名 | 動的モデル選択 |
| `{{lora_strength_model}}` | 数値 | LoRA モデル強度 | 動的モデル選択 |
| `{{lora_strength_clip}}` | 数値 | LoRA CLIP 強度 | 動的モデル選択 |
| `{{reference_image}}` | 文字列 | IP-Adapter 参照画像ファイル名 | コンセプト画像パス |
| `{{ipadapter_weight}}` | 数値 | IP-Adapter ウェイト | デフォルト: 0.8 |
| `{{direction}}` | 文字列 | 方向名 | 方向展開時の方向指定 |
| `{{animation_name}}` | 文字列 | アニメーション名 | アニメーション展開時のアニメーション種別 |
| `{{frame_index}}` | 数値 | フレームインデックス | アニメーション展開時のフレーム番号 |

**置換処理の仕様:**

- 文字列プレースホルダ: JSON 内の `{{key}}` をそのまま文字列値に置換
- 数値プレースホルダ: JSON 内の `"{{key}}"` を引用符ごと数値に置換（`"{{seed}}"` -> `12345`）
- 数値判定: `f64` または `i64` としてパース可能な場合に数値として扱う
- 置換後に無効な JSON が生成された場合は元のワークフローにフォールバック

**シード決定ロジック:**
- `ProcessingParams.seed` が指定されている場合: `base_seed = seed`
- 未指定の場合: 現在時刻（ミリ秒）から生成
- スプライトごとに `base_seed + index` でインクリメント（再現性確保）

**参照ファイル:** `src-tauri/src/commands/comfyui.rs`（`replace_placeholders` 関数）

---

### (3) BG Removal -- 背景除去

**コマンド:** `remove_background`, `remove_background_batch`
**ソース:** `src-tauri/src/commands/background_removal.rs`

ONNX Runtime 上の U2-Net モデルを使用して，スプライト画像の背景を透明化する．

**初期化フロー:**

```
1. check_onnx_model      # モデルダウンロード状態の確認
        ↓
2. download_onnx_model   # u2net.onnx をダウンロード（未DL時）
        ↓
3. initialize_onnx       # OnnxService を生成し AppState に格納
```

モデルファイルの保存先: `{app_data_dir}/models/u2net.onnx`

**バッチ処理フロー:**

```
1. DB からキャラクター・プロジェクト情報を取得
        ↓
2. 処理対象スプライトをフィルタ（raw_path or processed_path が存在）
        ↓
3. 出力ディレクトリ作成: {base_path}/{character_name}/bg_removed/
        ↓
4. 各スプライトについて:
   a. 入力画像パス決定（processed_path > raw_path）
   b. ONNX 推論による背景除去
   c. 結果画像を bg_removed/ に保存
   d. DB 更新: processed_path 設定，status → bg_removed
   e. bg-removal-progress イベント送信
        ↓
5. 処理結果（成功したパスの配列）を返却
```

**エラーハンドリング:**
- 個別のスプライト処理失敗はスキップし，バッチ全体を継続
- `bg-removal-progress` イベントで進捗を通知（`current`, `total`, `sprite_id`）

**参照ファイル:** `src-tauri/src/commands/background_removal.rs`, `src-tauri/src/services/onnx_service.rs`

---

### (4) Normalize -- 正規化

**コマンド:** `normalize_sprite`, `normalize_batch`
**ソース:** `src-tauri/src/commands/image_processing.rs`

背景除去後（またはそれ以前）の画像を，プロジェクト定義のタイルサイズに正規化する．

**処理手順:**

```
1. アルファチャンネルから不透明領域の BoundingBox を検出
        ↓
2. BoundingBox 領域を切り出し（crop）
        ↓
3. アスペクト比維持でタイルサイズに収まるようリサイズ（Lanczos3 フィルタ）
        ↓
4. タイルサイズ（tile_width x tile_height）の透明キャンバスを生成
        ↓
5. リサイズ画像を底辺揃え・水平中央配置でキャンバスに合成
        ↓
6. 結果を normalized/ に保存
```

**配置ルール:**
- 水平方向: 中央揃え（`offset_x = (tile_width - resized_width) / 2`）
- 垂直方向: 底辺揃え（`offset_y = tile_height - resized_height`）
- キャラクターの足元がタイル下端に固定される設計

**出力先:** `{base_path}/{character_name}/normalized/`

**DB 更新:** `final_path` を設定し，`status` を `finalized` に変更

**参照ファイル:** `src-tauri/src/commands/image_processing.rs`

---

### (5) Spritesheet -- スプライトシート生成

**コマンド:** `generate_spritesheet`
**ソース:** `src-tauri/src/commands/spritesheet.rs`

全 `finalized` 状態のスプライトを1枚の PNG 画像に配置する．

**レイアウト仕様:**

```
         idle[2]  walk[4]   attack[4]  hit[2]    ← アニメーション
         col 0-1  col 2-5   col 6-9    col 10-11
        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
 row 0  │   │   │   │   │   │   │   │   │   │   │   │   │  ← down
 (down) │   │   │   │   │   │   │   │   │   │   │   │   │
        ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
 row 1  │   │   │   │   │   │   │   │   │   │   │   │   │  ← left
 (left) │   │   │   │   │   │   │   │   │   │   │   │   │
        ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
 row 2  │   │   │   │   │   │   │   │   │   │   │   │   │  ← right
 (right)│   │   │   │   │   │   │   │   │   │   │   │   │
        ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
 row 3  │   │   │   │   │   │   │   │   │   │   │   │   │  ← up
 (up)   │   │   │   │   │   │   │   │   │   │   │   │   │
        └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
          ← 総列数 = Σ(各アニメーションのframe_count) = 12 →
```

**配置ルール:**
- 行: `directions` 配列の順序に対応（デフォルト: down, left, right, up）
- 列: `animations` 配列を連結（デフォルト: idle[2] + walk[4] + attack[4] + hit[2] = 12列）
- セルサイズ: `tile_width x tile_height`（デフォルト: 64x64）
- 不足フレーム（スプライトが存在しない or final_path がない）は透明セルとして埋め，`missing_frames` で報告

**出力:**
- 画像パス: `{base_path}/{character_name}/spritesheet/{character_name}.png`
- 画像サイズ: `(tile_width * total_columns) x (tile_height * direction_count)`

**スプライト検索:**
- HashMap `(direction, animation, frame_index)` をキーとして O(1) ルックアップ
- `finalized` ステータスかつ `final_path` が存在するスプライトのみ使用

**DB 更新:** `characters.spritesheet_path` に生成されたスプライトシートのパスを保存

**参照ファイル:** `src-tauri/src/commands/spritesheet.rs`

---

### (6) Bevy Export -- Bevy Engine エクスポート

**コマンド:** `export_for_bevy`
**ソース:** `src-tauri/src/commands/bevy_export.rs`

生成済みスプライトシートと，Bevy Engine で読み込むためのメタデータ JSON を出力する．

**処理フロー:**

```
1. キャラクター・プロジェクト情報を DB から取得
        ↓
2. spritesheet_path の存在確認
        ↓
3. ファイル名サニタイズ（path_utils::sanitize_filename）
        ↓
4. 出力先ディレクトリ作成: {output_dir}/characters/
        ↓
5. スプライトシート PNG をコピー: {safe_name}.png
        ↓
6. SpritesheetMeta を JSON として書き出し: {safe_name}.meta.json
        ↓
7. ExportResult を返却
```

**出力ファイル:**

```
{output_dir}/
└── characters/
    ├── {safe_name}.png          # スプライトシートPNG
    └── {safe_name}.meta.json    # メタデータJSON
```

**メタデータ JSON 構造（`meta.json`）:**

```json
{
  "version": "1.0",
  "character": "warrior",
  "spritesheet": {
    "columns": 12,
    "rows": 4,
    "tile_width": 64,
    "tile_height": 64
  },
  "directions": [
    { "name": "down",  "row": 0 },
    { "name": "left",  "row": 1 },
    { "name": "right", "row": 2 },
    { "name": "up",    "row": 3 }
  ],
  "animations": [
    { "name": "idle",   "start_col": 0,  "frame_count": 2, "frame_duration_ms": 300 },
    { "name": "walk",   "start_col": 2,  "frame_count": 4, "frame_duration_ms": 150 },
    { "name": "attack", "start_col": 6,  "frame_count": 4, "frame_duration_ms": 100 },
    { "name": "hit",    "start_col": 10, "frame_count": 2, "frame_duration_ms": 200 }
  ]
}
```

**メタデータ型（Rust）:**

| 型 | フィールド | 説明 |
|----|---------|------|
| `SpritesheetMeta` | `version`, `character`, `spritesheet`, `directions`, `animations` | ルート構造体 |
| `SpritesheetInfo` | `columns`, `rows`, `tile_width`, `tile_height` | シート寸法 |
| `DirectionMeta` | `name`, `row` | 方向と行のマッピング |
| `AnimationMeta` | `name`, `start_col`, `frame_count`, `frame_duration_ms` | アニメーション定義 |

**参照ファイル:** `src-tauri/src/commands/bevy_export.rs`

---

## ファイルシステム上のディレクトリ構造

### プロジェクトベースディレクトリ

`{base_path}` はプロジェクト作成時にユーザーが指定するベースパスである．

```
{base_path}/
├── {character_name_1}/
│   ├── concepts/               # (A1a-A1b) コンセプトアート・ピクセルアート画像
│   │   ├── concept_001.png
│   │   └── concept_002.png
│   ├── generated/              # (A2-A3) AI生成スプライト
│   │   ├── warrior_down_idle_00.png
│   │   └── ...
│   ├── raw/                    # (1) インポートされた元画像
│   │   ├── warrior_down_idle_00.png
│   │   ├── warrior_down_idle_01.png
│   │   └── ...
│   ├── ai_processed/           # (2) ComfyUI 質感変換後
│   │   ├── warrior_down_idle_00.png
│   │   └── ...
│   ├── bg_removed/             # (3) 背景除去後
│   │   ├── warrior_down_idle_00.png
│   │   └── ...
│   ├── normalized/             # (4) 正規化後（タイルサイズに統一）
│   │   ├── warrior_down_idle_00.png
│   │   └── ...
│   └── spritesheet/            # (5) 生成されたスプライトシート
│       └── warrior.png
├── {character_name_2}/
│   └── ...（同構造）
└── ...
```

### アプリケーションデータディレクトリ

`{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリである．

```
{app_data_dir}/
├── figurine_studio.db          # SQLite データベース
└── models/
    └── u2net.onnx              # U2-Net 背景除去モデル
```

### Bevy エクスポート先

`{output_dir}` はエクスポート時にユーザーが指定する出力先である．

```
{output_dir}/
└── characters/
    ├── warrior.png             # スプライトシート PNG
    ├── warrior.meta.json       # メタデータ JSON
    ├── slime.png
    ├── slime.meta.json
    └── ...
```

---

## 進捗イベント一覧

長時間処理はバックエンドから Tauri イベントを emit し，フロントエンドがリアルタイムで進捗を表示する．

| イベント名 | 送出元ステージ | ペイロード |
|-----------|--------------|----------|
| `concept-generation-progress` | (A1a-A1b) Concept Art / Pixel Art 生成 | `{ step, candidates_generated, status }` |
| `direction-generation-progress` | (A2) Direction 展開 | `{ current_direction, total_directions, status }` |
| `animation-generation-progress` | (A3) Animation 展開 | `{ direction, animation, current_frame, total_frames, status }` |
| `download-progress` | ONNX モデルダウンロード | `{ downloaded, total, percentage }` |
| `comfyui-progress` | AI Texture バッチ | `{ current, total, status, current_file }` |
| `bg-removal-progress` | BG Removal バッチ | `{ current, total, sprite_id }` |
| `normalize-progress` | Normalize バッチ | `{ current, total, sprite_id }` |
| `processing-progress` | 汎用処理進捗 | `{ stage, current, total }` |
