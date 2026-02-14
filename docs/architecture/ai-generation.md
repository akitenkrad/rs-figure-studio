# Figurine Studio -- AI キャラクター生成パイプライン

## 概要

Figurine Studio に追加される新しいキャラクター素材生成パスである．従来の MagicaVoxel 等からのインポートパスと**併用**される設計となっており，生成 AI を活用し，テキストプロンプトからスプライト素材を段階的に生成する．

---

## パイプライン全体図

```
┌─────────────────── AI 生成パス ─────────────────────────┐
│                                                          │
│  Step 1: コンセプト生成（1枚）                            │
│    テキストプロンプト → キャラクター原案画像               │
│         ↓                                                │
│  Step 2: 方向展開（4枚）                                  │
│    コンセプト画 + IP-Adapter + ControlNet → 4方向ポーズ   │
│         ↓                                                │
│  Step 3: アニメーション展開（方向×フレーム数）              │
│    各方向ポーズ + ControlNet → フレーム群                  │
│         ↓                                                │
└──→ BG Removal → Normalize → Spritesheet → Bevy Export   │
                                                           │
┌─── 従来パス（併用） ────────────────────────────────────┐│
│  MagicaVoxel等 → Import → (AI Texture) ─────────────→──┘│
└──────────────────────────────────────────────────────────┘
```

AI 生成パスで生成された画像は，Step 3 完了後に既存パイプラインの BG Removal ステージに合流する．従来のインポートパスとは独立して動作し，同一プロジェクト内で両方のパスを使い分けることができる．

---

## 技術選定

### 生成バックエンド

ユーザーの環境に応じて2つのバックエンドから選択可能とする．

#### 1. ComfyUI（ローカル GPU）

既存の ComfyUI 統合を拡張し，以下の構成でキャラクター素材を生成する:

- **コンセプト生成:** SDXL + Pixel Art XL LoRA による txt2img
- **方向展開:** IP-Adapter FaceID（コンセプト画像を参照）+ ControlNet（方向別 OpenPose スケルトン）
- **アニメーション展開:** IP-Adapter + ControlNet によるフレーム単位バッチ生成（seed 固定で一貫性確保）
- **ピクセルアート化:** 512x512 で生成後，PixelArt-Detector ノードでダウンスケール

**必要カスタムノード:**

| ノード | 用途 |
|--------|------|
| ComfyUI_IPAdapter_plus | IP-Adapter FaceID によるキャラクター参照 |
| comfyui_controlnet_aux | ControlNet 前処理（OpenPose / DWPose） |
| ComfyUI-PixelArt-Detector | ピクセルアートへのダウンスケール・グリッド整列 |

#### 2. クラウド API（GPU 不要）

ローカル GPU を持たないユーザー向けに，クラウド API バックエンドを提供する:

| サービス | 料金目安 | 特徴 |
|----------|---------|------|
| PixelLab | $12/月 | ピクセルアート専用 API，ワンクリック方向回転 |
| FLUX.2 via fal.ai | ~$0.03/枚 | マルチリファレンスによるキャラクター一貫性 |
| Retro Diffusion via Replicate | ~$0.01-0.05/回 | ピクセルアートアニメーション特化 |

---

### キャラクター一貫性手法: トリプルアプローチ

複数方向・複数フレームにわたってキャラクターの外見を一貫させるため，3つの手法を組み合わせる．

#### 1. IP-Adapter FaceID

推論時にキャラクターの特徴を維持する手法．追加学習が不要であり，コンセプト画像を参照画像として入力するだけで利用できる．weight は 0.7-0.85 の範囲で設定する．

#### 2. キャラクター LoRA

体型・衣装スタイルを高精度に再現するための手法．15-20枚の参照画像で学習し，学習時間は 5-15分程度である．外部ツールで学習した LoRA モデルをインポートして使用する．

#### 3. ControlNet

ポーズ・空間構図を制御する手法．OpenPose / DWPose のスケルトンデータを入力とし，方向別・フレーム別の姿勢を正確に指定する．

**一貫性手法の比較:**

| 手法 | 学習要否 | セットアップ時間 | 一貫性精度 | 用途 |
|------|---------|----------------|-----------|------|
| IP-Adapter FaceID | 不要 | 即時 | 中〜高 | 方向展開・フレーム生成時の参照 |
| キャラクター LoRA | 必要（5-15分） | 中 | 高 | 体型・衣装の高精度再現 |
| ControlNet | 不要 | 即時 | 高（ポーズのみ） | ポーズ・構図の空間制御 |

---

### ピクセルアート最適化

生成 AI の出力をゲーム向けピクセルアートとして使用するための最適化手法:

- **生成解像度:** 512x512 で生成し，ブロックモード色選択でダウンスケール
- **パレット正規化:** クラスタリングベースの色量子化により，統一パレットを適用
- **ピクセルグリッド整列:** ComfyUI PixelArt-Detector ノード，または Rust image crate による後処理
- **実用品質の目安:** 128x128 以上で良好（70-80%），256x256 で最良

---

## 各ステップの処理詳細

### Step 1: コンセプト生成

テキストプロンプトから1枚のキャラクター原案画像を生成する．

**Input:**
- テキストプロンプト（キャラクターの外見・スタイルの記述）
- スタイルプリセット（optional: ピクセルアート，ドット絵，等）

**処理パス:**

| バックエンド | 処理内容 |
|-------------|---------|
| ComfyUI | SDXL + Pixel Art XL LoRA による txt2img |
| Cloud API | PixelLab / fal.ai / Replicate API コール |

**Output:**
- 1枚のコンセプト画像
- 保存先: `{base_path}/{character_name}/concepts/`

ユーザーが結果を確認し，満足のいく結果が得られるまでリトライが可能である．採用するコンセプト画像を選別したうえで次のステップに進む．

---

### Step 2: 方向展開

コンセプト画像を基に，各方向（デフォルト: down, left, right, up）の基本ポーズ画像を生成する．

**Input:**
- コンセプト画像（Step 1 で選択したもの）
- ターゲット方向リスト（デフォルト: down, left, right, up）

**処理パス:**

| バックエンド | 処理内容 |
|-------------|---------|
| ComfyUI | IP-Adapter（コンセプト画像を参照）+ ControlNet（方向別 OpenPose スケルトン） |
| Cloud API | PixelLab rotation API / fal.ai マルチリファレンス生成 |

**一貫性確保:**
- Seed 固定により生成のばらつきを抑制
- 同一 IP-Adapter 参照画像を全方向で共有

**Output:**
- 方向数 x 1枚の基本ポーズ画像
- 保存先: `{base_path}/{character_name}/generated/`

---

### Step 3: アニメーション展開

各方向の基本ポーズ画像を基に，アニメーションフレームを生成する．

**Input:**
- 各方向の基本ポーズ画像（Step 2 の出力）
- ターゲットアニメーションとフレーム数（例: idle[2], walk[4], attack[4], hit[2]）

**処理パス:**

| バックエンド | 処理内容 |
|-------------|---------|
| ComfyUI | IP-Adapter（基本ポーズ画像を参照）+ ControlNet（フレーム別ポーズスケルトン），seed 固定のフレーム単位バッチ生成 |
| Cloud API | Retro Diffusion アニメーションプリセット / PixelLab animation API |

**Output:**
- 方向 x アニメーション x フレーム数 の画像群
- 保存先: `{base_path}/{character_name}/generated/`
- 既存パイプラインの `sprites` テーブルに `raw` ステータスで登録

---

## ステータス遷移（拡張）

AI 生成パスの導入に伴い，スプライトのステータス遷移に `generated` ステータスを追加する．

```
                     ┌── AI生成パス ──┐
                     │                │
                     ▼                │
generated ──→ raw ──→ ai_processed ──→ bg_removed ──→ finalized
                │         │                │
                │         │                └──→ finalized
                │         └──→ finalized
                └──→ finalized
```

`generated` は AI 生成パスで生成されたスプライトの初期ステータスである．ユーザーによる確認・選別を経て `raw` に遷移し，以降は従来パイプラインと同一のフローで処理される．

**参照ファイル:** `src-tauri/src/models/sprite.rs`（`SpriteStatus` 列挙型 -- 拡張対象）

---

## ディレクトリ構造（拡張）

AI 生成パスの導入に伴い，キャラクターディレクトリに `concepts/` と `generated/` サブディレクトリを追加する．

```
{base_path}/{character_name}/
├── concepts/             # Step 1: コンセプト画像
│   ├── concept_001.png
│   └── concept_002.png
├── generated/            # Step 2-3: AI生成スプライト
│   ├── warrior_down_idle_00.png
│   └── ...
├── raw/                  # 従来パス: インポート元画像
├── ai_processed/         # ComfyUI 質感変換後
├── bg_removed/           # 背景除去後
├── normalized/           # 正規化後
└── spritesheet/          # スプライトシート
```

---

## 生成バックエンド抽象レイヤー

ComfyUI とクラウド API を統一的に扱うための抽象レイヤーを設計する．

```rust
// GenerationBackend trait（概念設計）
trait GenerationBackend {
    async fn generate_concept(&self, params: ConceptParams) -> Result<Vec<PathBuf>, AppError>;
    async fn generate_directions(&self, concept: &Path, params: DirectionParams) -> Result<Vec<PathBuf>, AppError>;
    async fn generate_animation_frames(&self, base_pose: &Path, params: AnimationParams) -> Result<Vec<PathBuf>, AppError>;
}
```

**実装クラス:**

| 実装 | ソースファイル | 説明 |
|------|-------------|------|
| `ComfyUIBackend` | `src-tauri/src/services/comfyui_client.rs` | 既存 ComfyUI HTTP クライアントを拡張 |
| `CloudAPIBackend` | **新規** `src-tauri/src/services/cloud_api_client.rs` | PixelLab / fal.ai / Replicate クライアント |

ユーザーが設定画面でバックエンドを選択し，`app_settings` テーブルに保存する．ランタイムでは選択されたバックエンドに応じて適切な実装が使用される．

---

## ワークフローテンプレート

ComfyUI バックエンド用に，以下のプリビルトワークフロー JSON テンプレートを提供する．各テンプレートはプレースホルダ（`{{key}}` 形式）を含み，実行時にパラメータに置換される．

| テンプレート | 用途 |
|-------------|------|
| `concept_generation.json` | txt2img with Pixel Art LoRA によるコンセプト生成 |
| `direction_expansion.json` | IP-Adapter + ControlNet による多方向展開 |
| `animation_frame.json` | IP-Adapter + ControlNet によるフレーム単位生成（seed 固定） |
| `post_process_pixelart.json` | パレット量子化 + グリッド正規化によるピクセルアート後処理 |

テンプレートはアプリ内にバンドルされ，ユーザーがカスタムワークフローとして編集・保存することも可能とする．

---

## ProcessingParams 拡張

既存の `ProcessingParams` に以下のフィールドを追加し，AI 生成パイプラインのパラメータを受け渡す．

| 追加フィールド | 型 | 説明 |
|---|---|---|
| `reference_image_path` | `Option<String>` | IP-Adapter 参照画像パス |
| `ipadapter_weight` | `Option<f64>` | IP-Adapter ウェイト（デフォルト: 0.8） |
| `lora_name` | `Option<String>` | 使用する LoRA モデル名 |
| `lora_weight` | `Option<f64>` | LoRA ウェイト（デフォルト: 1.0） |

**参照ファイル:** `src-tauri/src/models/workflow.rs`（`ProcessingParams` 構造体 -- 拡張対象）

---

## 段階的導入計画

| Phase | 内容 | 優先度 |
|---|---|---|
| Phase 1 | コンセプト生成（ComfyUI ワークフローテンプレート + UI） | 高 |
| Phase 2 | 方向展開（IP-Adapter + ControlNet） | 高 |
| Phase 3 | アニメーション展開（フレーム単位バッチ生成） | 高 |
| Phase 4 | クラウド API クライアント（PixelLab / fal.ai） | 中 |
| Phase 5 | パレット正規化・ピクセルグリッド整列ポスト処理 | 中 |
| Phase 6 | LoRA 管理（外部学習 → インポート） | 低 |

Phase 1-3 は ComfyUI バックエンドを前提とし，ローカル GPU 環境での動作を優先する．Phase 4 以降でクラウド API バックエンドを追加し，GPU 不要の環境にも対応する．

---

## 参考文献・ツール

- **Sprite Sheet Diffusion** (arXiv 2412.03685) -- ポーズ条件付きスプライト生成の研究
- **VNCCS** (ComfyUI plugin) -- 5段階キャラクター一貫性パイプライン
- **PixelLab** -- ピクセルアート専用 AI サービス
- **Retro Diffusion** -- ピクセルアート特化 AI モデル

---

## 関連ソースファイル

| ファイル | 役割 |
|---|---|
| `src-tauri/src/services/comfyui_client.rs` | 既存 ComfyUI HTTP クライアント（拡張対象） |
| `src-tauri/src/models/workflow.rs` | ProcessingParams（拡張対象） |
| **新規** `src-tauri/src/services/cloud_api_client.rs` | クラウド API クライアント |
| **新規** `src-tauri/src/services/generation_backend.rs` | 生成バックエンド抽象レイヤー |
