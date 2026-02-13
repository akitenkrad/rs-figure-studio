# ONNX Runtime / U2-Net ガイド

## 概要

Figurine Studio は，ONNX Runtime と U2-Net モデルを使用してスプライト画像の背景除去を行う．背景除去はスプライトパイプラインの「BG Removal」ステップに位置し，AI テクスチャ生成（ComfyUI）の後，スプライトシート合成の前に実行される．

### スプライトパイプラインにおける位置づけ

```
Import (raw) → AI Texture (ComfyUI) → [BG Removal (ONNX)] → Normalize → Spritesheet → Bevy Export
                                        ↑ この部分が ONNX/U2-Net
```

### 関連ソースファイル

| ファイル | 役割 |
|---------|------|
| `src-tauri/src/services/onnx_service.rs` | ONNX 推論サービス（前処理・推論・後処理） |
| `src-tauri/src/commands/model_download.rs` | モデルダウンロード・存在確認コマンド |
| `src-tauri/src/commands/background_removal.rs` | 背景除去コマンド（単体・バッチ） |
| `src-tauri/src/state.rs` | `AppState`（ONNX サービスの遅延初期化保持） |
| `src-tauri/Cargo.toml` | `ort` クレートの依存定義 |

---

## モデル仕様

### U2-Net

| 項目 | 値 |
|------|-----|
| モデル名 | U2-Net (U^2-Net) |
| タスク | Salient Object Detection（顕著物体検出） |
| ファイル名 | `u2net.onnx` |
| ファイルサイズ | 約170MB |
| 入力テンソル | `[1, 3, 320, 320]` (NCHW，float32) |
| 出力テンソル | `[1, 1, 320, 320]` (NCHW，float32，確率マスク) |

U2-Net は前景/背景を分離するためのセグメンテーションモデルであり，ピクセルアート・レンダリング画像のキャラクター切り抜きに使用される．

### ダウンロード元 URL

```
https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx
```

[rembg](https://github.com/danielgatis/rembg) プロジェクトの GitHub Releases からダウンロードされる．

---

## モデルの保存と管理

### 保存先パス

```
{app_data_dir}/models/u2net.onnx
```

`{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリである．

| プラットフォーム | 典型的なパス |
|---------------|------------|
| macOS | `~/Library/Application Support/com.figurine-studio.app/models/u2net.onnx` |
| Windows | `C:\Users\{user}\AppData\Roaming\com.figurine-studio.app\models\u2net.onnx` |
| Linux | `~/.local/share/com.figurine-studio.app/models/u2net.onnx` |

### ディレクトリ構造

```
{app_data_dir}/
├── figurine_studio.db    ← SQLite データベース
└── models/
    ├── u2net.onnx        ← ダウンロード済みモデル
    └── u2net.onnx.tmp    ← ダウンロード中の一時ファイル（完了後に削除）
```

`models/` ディレクトリは `AppState::new()` の初期化時に `create_dir_all` で自動作成される．

### アトミックリネーム方式

モデルダウンロードでは，ファイル破損を防ぐためにアトミックリネーム方式を採用している．

```
[ダウンロード開始]
  │
  ├─ 一時ファイルに書き込み: {app_data_dir}/models/u2net.onnx.tmp
  ├─ ストリーミングダウンロード（チャンクごとに書き込み）
  ├─ フラッシュ & クローズ
  ├─ ファイルサイズ検証（0バイトでないことを確認）
  ├─ アトミックリネーム: u2net.onnx.tmp → u2net.onnx
  └─ DB 設定更新: onnx_model_downloaded=true, onnx_model_path=...

[エラー時]
  └─ ファイルサイズが 0 の場合: 一時ファイルを削除してエラー返却
```

この方式により，ダウンロード途中でアプリがクラッシュしても，不完全な `.onnx` ファイルが残ることがない．正式パスには完全なファイルのみが配置される．

### ダウンロード済み判定

`check_onnx_model` コマンドで以下の判定が行われる．

1. `{app_data_dir}/models/u2net.onnx` のファイル存在チェック
2. 存在する場合: `downloaded: true` とファイルサイズを返却
3. 存在しない場合: DB フラグ `onnx_model_downloaded` を `"false"` にリセットし，`downloaded: false` を返却

### ダウンロードプログレスイベント

ダウンロード中は `model-download-progress` イベントでフロントエンドに進捗を通知する．

```json
{
  "downloaded_bytes": 85000000,
  "total_bytes": 170000000,
  "percentage": 50
}
```

`total_bytes` はレスポンスの `Content-Length` ヘッダから取得する．ヘッダが存在しない場合は `0` となり，`percentage` も `0` が返る．

---

## ONNX セッション設定

### セッション構築パラメータ

`OnnxService::new()` で ONNX Runtime セッションを以下の設定で構築する．

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| GraphOptimizationLevel | `Level3` | 最高レベルの最適化（ノード融合・定数畳み込み等） |
| intra_threads | `4` | 単一オペレータ内の並列スレッド数 |

```rust
Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?
    .commit_from_file(model_path)?;
```

### 遅延初期化

ONNX セッションはアプリ起動時には生成されず，フロントエンドから明示的に `initialize_onnx` コマンドが呼び出されたタイミングで初期化される．

```
AppState {
    onnx_service: Mutex<Option<OnnxService>>  ← 初期値: None
}
```

初期化フロー:

```
[アプリ起動]
  └─ onnx_service = None

[ユーザーが背景除去機能を使用]
  ├─ check_onnx_model → モデル存在確認
  ├─ download_onnx_model → （未ダウンロードの場合）ダウンロード
  ├─ initialize_onnx → OnnxService::new() でセッション生成，AppState に格納
  └─ remove_background / remove_background_batch → 推論実行
```

この遅延初期化により，背景除去を使用しないユーザーに対して ONNX Runtime のロードコストやメモリ消費を回避している．

---

## 推論パイプライン

背景除去の推論パイプラインは「前処理 → 推論 → 後処理」の3段階で構成される．

### 1. 前処理（`preprocess` 関数）

入力画像を ONNX モデルが受け付けるテンソル形式に変換する．

| ステップ | 処理内容 |
|---------|---------|
| 1 | 320x320 にリサイズ（Bilinear フィルタ: `FilterType::Triangle`） |
| 2 | RGB 形式に変換（アルファチャンネルがある場合は除去） |
| 3 | ピクセル値を `[0, 255]` から `[0.0, 1.0]` に正規化 |
| 4 | ImageNet 統計量でチャンネルごとに正規化 |
| 5 | NCHW 形式 `[1, 3, 320, 320]` の `Array4<f32>` テンソルに変換 |

#### ImageNet 正規化パラメータ

| チャンネル | Mean | Std |
|-----------|------|-----|
| R | 0.485 | 0.229 |
| G | 0.456 | 0.224 |
| B | 0.406 | 0.225 |

正規化計算:

```
normalized = (pixel / 255.0 - mean) / std
```

### 2. 推論

前処理済みテンソルを ONNX セッションに入力し，セグメンテーションマスクを取得する．

- **入力**: `[1, 3, 320, 320]` float32 テンソル
- **出力**: `[1, 1, 320, 320]` float32 テンソル（各ピクセルの前景確率 0.0〜1.0）
- **出力テンソル**: 複数出力がある場合，最初の出力（インデックス 0）を使用

出力テンソルのサイズ検証:

```rust
let expected_elements = 1 * 1 * 320 * 320;  // = 102,400
if data.len() < expected_elements {
    // エラー: 予期しない出力テンソルサイズ
}
```

### 3. 後処理（`postprocess` 関数）

ONNX 出力マスクをアルファチャンネルとして元画像に適用する．

| ステップ | 処理内容 |
|---------|---------|
| 1 | マスクデータ（フラット配列）を 320x320 のグレースケール画像に変換（値を `[0.0, 1.0]` → `[0, 255]` にクランプ） |
| 2 | グレースケールマスクを元画像サイズにリサイズ（Bilinear フィルタ） |
| 3 | 元画像を RGBA 形式に変換 |
| 4 | 閾値処理でアルファチャンネルを設定 |

#### マスク閾値処理（デフォルト閾値: 0.5）

```
if mask_value < threshold:
    alpha = 0          (完全透明 = 背景)
else:
    alpha = mask_value * 255  (ソフトエッジ = 前景の確信度に応じた透明度)
```

閾値以上のピクセルには，マスク値に比例したアルファ値が設定される．これにより，キャラクターの輪郭部分で滑らかな半透明のエッジが生成される．

### パイプラインの図解

```
[入力画像]                    [320x320 テンソル]        [確率マスク]           [RGBA 出力]
 任意サイズ  → リサイズ+正規化 → [1,3,320,320] → ONNX → [1,1,320,320] → 後処理 → 背景透明PNG
 (RGB/RGBA)    (ImageNet stats)   (float32)    推論    (float32 0~1)  (閾値0.5)  (元サイズ)
```

---

## バッチ処理

### 逐次処理方式

バッチ背景除去は逐次的に処理される．ONNX 推論は CPU 集約的であり，`Mutex<Option<OnnxService>>` で排他制御されているため，並列実行は行わない．

### `remove_background_batch` コマンドのフロー

```
[事前準備]
  ├─ DB からキャラクター・プロジェクト・スプライト一覧を取得
  ├─ 処理対象: raw_path または processed_path が存在するスプライト
  └─ 出力ディレクトリ作成: {base_path}/{character_name}/bg_removed/

[スプライトごとの処理（逐次）]
  ├─ 入力パス決定: processed_path > raw_path の優先順
  ├─ ONNX 推論実行（Mutex ロック取得 → 背景除去 → ロック解放）
  ├─ 成功時: DB 更新（processed_path 設定，status → bg_removed）
  ├─ 失敗時: ログ警告を出力し，スキップして継続
  └─ プログレスイベント送信: bg-removal-progress

[完了]
  └─ 成功したスプライトの出力パス一覧を返却
```

### 単体処理 (`remove_background` コマンド)

単一スプライトの背景除去も可能．フローはバッチ処理と同様だが，1枚のみの処理となる．

### 入出力パス

| パス | 説明 |
|------|------|
| 入力 | `sprite.processed_path` または `sprite.raw_path` |
| 出力 | `{base_path}/{character_name}/bg_removed/{filename}` |

### プログレスイベント

バッチ処理中は `bg-removal-progress` イベントでフロントエンドに進捗を通知する．

```json
{
  "current": 5,
  "total": 20,
  "sprite_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
}
```

### OnnxService のバッチ API

`OnnxService` 自体にも `remove_background_batch` メソッドがあり，`on_progress` コールバックで進捗を通知できる．ただし，コマンド層（`commands/background_removal.rs`）では DB 更新やイベント送信のために個別にループを回している．

---

## ort クレートのバージョンと features

### Cargo.toml の依存定義

```toml
ort = { version = "2.0.0-rc.11", features = ["download-binaries"] }
```

| 項目 | 値 |
|------|-----|
| クレート名 | `ort` |
| バージョン | `2.0.0-rc.11`（リリース候補版） |
| feature フラグ | `download-binaries` |

### `download-binaries` feature

この feature を有効にすると，ビルド時に ONNX Runtime の事前コンパイル済みバイナリ（共有ライブラリ）が自動的にダウンロードされる．これにより，開発者が ONNX Runtime を手動でインストールする必要がなくなる．

ダウンロードされるバイナリ:

| プラットフォーム | ライブラリ |
|---------------|-----------|
| macOS | `libonnxruntime.dylib` |
| Windows | `onnxruntime.dll` |
| Linux | `libonnxruntime.so` |

### 関連する依存クレート

| クレート | バージョン | 用途 |
|---------|-----------|------|
| `ndarray` | 0.17 | テンソルデータの操作（前処理の NCHW 変換） |
| `image` | 0.25 | 画像読み込み・リサイズ・保存 |

---

## unsafe Send+Sync の説明

### 実装

```rust
unsafe impl Send for OnnxService {}
unsafe impl Sync for OnnxService {}
```

### 背景と理由

`OnnxService` は `ort::Session` を内部に保持している．`ort::Session` は Rust の型システム上 `Send` および `Sync` を自動実装していない．しかし，以下の理由から手動で `unsafe impl` している．

1. **C++ ランタイムのスレッドセーフ性**: ONNX Runtime の C++ 実装は内部的にスレッドセーフに設計されている．`Session::Run()` は内部でロックを取得するため，複数スレッドからの同時呼び出しが安全である．

2. **Tauri の要求**: Tauri のコマンドハンドラは `async` であり，`tauri::State<'_, AppState>` を通じて共有状態にアクセスする．`AppState` が `Send + Sync` であるためには，そのフィールドである `Mutex<Option<OnnxService>>` も `Send + Sync` である必要がある．`Mutex` は内部型が `Send` であれば `Send + Sync` を実装するため，`OnnxService` に `Send` が必要である．

3. **Mutex による排他制御**: 実際のアクセスは `Mutex<Option<OnnxService>>` を通じて行われるため，同時に複数のスレッドが `OnnxService` のメソッドを呼び出すことはない．`Mutex` のロック取得により排他性が保証される．

### 安全性の根拠

```
AppState
  └─ onnx_service: Mutex<Option<OnnxService>>
                    ^^^^^ 排他制御
                           ^^^^^^^^^^^^^^
                           OnnxService { session: ort::Session }
                                                  ^^^^^^^^^^^^
                                                  C++ ONNX Runtime（内部的にスレッドセーフ）
```

`Mutex` による排他制御と ONNX Runtime C++ 実装のスレッドセーフ性の組み合わせにより，この `unsafe impl` は実用上安全である．

---

## トラブルシューティング

### モデルダウンロードが失敗する

| 確認項目 | 対処 |
|---------|------|
| ネットワーク接続 | インターネット接続を確認．GitHub へのアクセスが可能か確認 |
| プロキシ環境 | 企業プロキシ環境の場合，GitHub Releases へのアクセスがブロックされる可能性がある |
| ディスク容量 | `{app_data_dir}/models/` に約170MBの空き容量が必要 |
| 一時ファイルの残留 | `u2net.onnx.tmp` が残っている場合は手動で削除 |

### ONNX 初期化が失敗する

| 確認項目 | 対処 |
|---------|------|
| モデルファイルの破損 | `u2net.onnx` を削除して再ダウンロード |
| ONNX Runtime バイナリ | ビルド時に `download-binaries` で取得された共有ライブラリが正しく配置されているか確認 |
| メモリ不足 | ONNX セッション生成時に約170MBのメモリが必要 |

### 背景除去の品質が低い

| 確認項目 | 対処 |
|---------|------|
| 入力画像の品質 | 高解像度・高コントラストの画像ほど精度が高い |
| 閾値の調整 | デフォルト閾値は 0.5．`remove_background_with_threshold` で調整可能（0.0〜1.0） |
| モデルの限界 | U2-Net は写実的な画像に最適化されているため，極端にスタイライズされたピクセルアートでは精度が低下する場合がある |
