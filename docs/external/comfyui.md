# ComfyUI 連携ガイド

## 概要

Figurine Studio は，ComfyUI と連携して AI テクスチャ生成（質感変換）を行う機能を備えている．この機能は**オプション依存**であり，ComfyUI がインストール・起動されていなくてもアプリケーション本体は正常に動作する．ComfyUI 連携はスプライトパイプラインの「AI Texture」ステップでのみ使用される．

### スプライトパイプラインにおける位置づけ

```
Import (raw) → [AI Texture (ComfyUI)] → BG Removal (ONNX) → Normalize → Spritesheet → Bevy Export
                ↑ この部分が ComfyUI 連携
```

### 関連ソースファイル

| ファイル | 役割 |
|---------|------|
| `src-tauri/src/services/comfyui_client.rs` | HTTP クライアント実装 |
| `src-tauri/src/commands/comfyui.rs` | Tauri コマンド（フロントエンド API） |
| `src-tauri/src/models/workflow.rs` | ワークフロー・処理パラメータモデル |
| `src-tauri/tauri.conf.json` | CSP（Content Security Policy）設定 |

---

## 接続設定

### デフォルトエンドポイント

```
http://127.0.0.1:8188
```

エンドポイントはアプリ設定（`app_settings` テーブルの `comfyui_endpoint` キー）で変更可能．未設定の場合は上記デフォルト値が使用される．

### CSP（Content Security Policy）設定

`src-tauri/tauri.conf.json` の `app.security.csp` で，ComfyUI への接続が明示的に許可されている．

```
connect-src 'self' http://127.0.0.1:8188 ws://127.0.0.1:8188
```

| ディレクティブ | 許可対象 | 用途 |
|--------------|---------|------|
| `http://127.0.0.1:8188` | HTTP 接続 | REST API 呼び出し |
| `ws://127.0.0.1:8188` | WebSocket 接続 | 将来の拡張用（現在は未使用） |

> **注意**: ComfyUI のリッスンアドレスやポートを変更した場合は，`tauri.conf.json` の CSP 設定も合わせて更新する必要がある．

### HTTP クライアント設定

`ComfyUIClient` は以下の設定で `reqwest::Client` を生成する．

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| タイムアウト | 30秒 | リクエスト全体のタイムアウト |
| コネクションプール | 最大5接続/ホスト | アイドル接続の再利用 |
| クライアント ID | UUID v4 | セッションごとに自動生成 |

ヘルスチェック（`/system_stats`）のみ個別に5秒タイムアウトが設定されている．

---

## API エンドポイント一覧

ComfyUI REST API の以下5つのエンドポイントを使用する．

### 1. `GET /system_stats` -- サーバー生存確認

ヘルスチェックに使用する．HTTP ステータスコード 2xx が返れば接続成功と判定する．

```
→ GET http://127.0.0.1:8188/system_stats
← 200 OK (JSON: システム情報)
```

- **タイムアウト**: 5秒（専用設定）
- **失敗時**: `false` を返却（エラーを伝播しない）
- **呼び出し元**: `check_comfyui_connection` コマンド，バッチ処理開始前の事前チェック

### 2. `POST /upload/image` -- 画像アップロード

入力画像を ComfyUI のワーキングディレクトリにアップロードする．

```
→ POST http://127.0.0.1:8188/upload/image
  Content-Type: multipart/form-data
  Fields:
    image: (バイナリ, MIME: image/png)
    subfolder: "input"
    overwrite: "true"

← 200 OK
  {
    "name": "uploaded_filename.png",
    "subfolder": "input",
    "type": "input"
  }
```

- **リトライ**: あり（最大3回，Exponential Backoff）
- **サブフォルダ**: 常に `"input"` を指定

### 3. `POST /prompt` -- ワークフロー実行キュー投入

プレースホルダ置換済みのワークフロー JSON をキューに投入する．

```
→ POST http://127.0.0.1:8188/prompt
  Content-Type: application/json
  {
    "prompt": { ... (ワークフロー JSON) },
    "client_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
  }

← 200 OK
  {
    "prompt_id": "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
  }
```

- **リトライ**: あり（最大3回，Exponential Backoff）
- **client_id**: `ComfyUIClient` 生成時に割り当てられた UUID v4

### 4. `GET /history/{prompt_id}` -- 実行結果取得

指定された `prompt_id` の実行履歴・結果を取得する．

```
→ GET http://127.0.0.1:8188/history/{prompt_id}

← 200 OK
  {
    "{prompt_id}": {
      "status": {
        "completed": true,
        "status_str": "success"
      },
      "outputs": {
        "{node_id}": {
          "images": [
            {
              "filename": "ComfyUI_00001_.png",
              "subfolder": "",
              "type": "output"
            }
          ]
        }
      }
    }
  }
```

- **リトライ**: なし（ポーリングループ内で繰り返し呼び出されるため）
- **判定ロジック**:
  - `status.completed == true` かつ `status.status_str != "error"` で成功
  - `status.completed == true` かつ `status.status_str == "error"` でエラー
  - `status.completed == false` または該当エントリなしで処理中

### 5. `GET /view` -- 結果画像ダウンロード

ComfyUI が生成した出力画像をバイナリで取得する．

```
→ GET http://127.0.0.1:8188/view?filename={name}&subfolder={subfolder}&type=output

← 200 OK (image/png バイナリ)
```

- **リトライ**: あり（最大3回，Exponential Backoff）
- **クエリパラメータ**: 簡易 URL エンコーディング（RFC 3986 非予約文字以外をパーセントエンコード）

---

## リトライ戦略

画像アップロード，キュー投入，画像ダウンロードの3つの操作に Exponential Backoff リトライが適用される．

### パラメータ

| 項目 | 値 |
|------|-----|
| 最大リトライ回数 | 3回（初回 + リトライ3回 = 合計4回試行） |
| 初回リトライ待機 | 1秒 (`2^0`) |
| 2回目リトライ待機 | 2秒 (`2^1`) |
| 3回目リトライ待機 | 4秒 (`2^2`) |

### リトライ対象

| 操作 | リトライ | 理由 |
|------|---------|------|
| `upload_image` | あり | ネットワーク一時障害への耐性 |
| `queue_prompt` | あり | サーバー負荷時のエラー対策 |
| `get_image` | あり | 大容量画像転送の安定性確保 |
| `get_history` | なし | ポーリングで繰り返すため不要 |
| `health_check` | なし | 単発チェックのため不要 |

### 実装詳細

```rust
// with_retry 関数のバックオフ計算
let delay = Duration::from_secs(2u64.pow(attempt));
// attempt=0: 1秒, attempt=1: 2秒, attempt=2: 4秒
```

全リトライが失敗した場合，最後のエラーが呼び出し元に伝播される．

---

## ワークフロープレースホルダ仕様

ComfyUI ワークフロー JSON 内の `{{key}}` 形式のプレースホルダが，実行時に実際の値に置換される．

### プレースホルダ一覧（8種）

| プレースホルダ | 型 | 説明 | 値の出所 |
|--------------|-----|------|---------|
| `{{input_image}}` | 文字列 | アップロード済み画像のファイル名 | `upload_image` のレスポンス |
| `{{positive_prompt}}` | 文字列 | ポジティブプロンプト | `ProcessingParams.positive_prompt` |
| `{{negative_prompt}}` | 文字列 | ネガティブプロンプト | `ProcessingParams.negative_prompt` |
| `{{seed}}` | 数値 | 乱数シード | `ProcessingParams.seed` またはタイムスタンプ由来 |
| `{{steps}}` | 数値 | サンプリングステップ数 | `ProcessingParams.steps` |
| `{{cfg_scale}}` | 数値 | CFG スケール | `ProcessingParams.cfg_scale` |
| `{{denoise_strength}}` | 数値 | デノイズ強度 | `ProcessingParams.denoise_strength` |
| `{{controlnet_weight}}` | 数値 | ControlNet ウェイト | `ProcessingParams.controlnet_weight` |

### 置換ルール

1. **文字列プレースホルダ**: `{{key}}` をそのまま値文字列に置換
2. **数値プレースホルダ**: JSON 内で `"{{key}}"` のように引用符付きで記述されている場合，引用符ごと数値に置換（有効な JSON 数値リテラルになるように）

```json
// 置換前
{
  "inputs": {
    "image": "{{input_image}}",
    "seed": "{{seed}}",
    "steps": "{{steps}}"
  }
}

// 置換後
{
  "inputs": {
    "image": "uploaded_file.png",
    "seed": 1707849600000,
    "steps": 20
  }
}
```

### seed の決定ロジック

- `ProcessingParams.seed` が指定されている場合: その値を基準シードとして使用
- 未指定の場合: `SystemTime::now()` のミリ秒エポックタイムを基準シードとして使用
- バッチ処理時: 各スプライトに `base_seed + index` が割り当てられる（インデックスは0始まり）

### ワークフロー JSON の例

ComfyUI からエクスポートした API 形式 JSON にプレースホルダを埋め込む．

```json
{
  "3": {
    "class_type": "KSampler",
    "inputs": {
      "seed": "{{seed}}",
      "steps": "{{steps}}",
      "cfg": "{{cfg_scale}}",
      "denoise": "{{denoise_strength}}",
      "positive": ["6", 0],
      "negative": ["7", 0],
      "model": ["4", 0],
      "latent_image": ["5", 0]
    }
  },
  "6": {
    "class_type": "CLIPTextEncode",
    "inputs": {
      "text": "{{positive_prompt}}",
      "clip": ["4", 1]
    }
  },
  "7": {
    "class_type": "CLIPTextEncode",
    "inputs": {
      "text": "{{negative_prompt}}",
      "clip": ["4", 1]
    }
  }
}
```

---

## バッチ処理フロー

`process_batch_comfyui` コマンドは，指定キャラクターの全スプライトに対して逐次的に ComfyUI 質感変換を実行する．

### 全体フロー

```
[事前チェック]
  ├─ DB からキャラクター・プロジェクト・スプライト・ワークフロー情報を取得
  ├─ ComfyUI エンドポイント取得（app_settings から，デフォルト: http://127.0.0.1:8188）
  ├─ ヘルスチェック（GET /system_stats）
  └─ 出力ディレクトリ作成: {base_path}/{character_name}/ai_processed/

[スプライトごとの処理（逐次）]
  ├─ 1. Upload:   POST /upload/image（入力画像を ComfyUI に送信）
  ├─ 2. Replace:  プレースホルダ置換（8種のパラメータを埋め込み）
  ├─ 3. Queue:    POST /prompt（ワークフロー実行をキューに投入）
  ├─ 4. Poll:     GET /history/{prompt_id}（完了までポーリング）
  ├─ 5. Download: GET /view（結果画像をダウンロード）
  └─ 6. Save:     ローカルファイルに保存 + DB 更新（status → ai_processed）

[後処理]
  └─ processing_jobs テーブルの更新（completed / partial / failed）
```

### 処理対象スプライトの選定

`raw_path` または `processed_path` が存在するスプライトのみが処理対象となる．入力画像の優先順位は `processed_path` > `raw_path`．

### 入出力パス

| パス | 説明 |
|------|------|
| 入力 | `sprite.processed_path` または `sprite.raw_path` |
| 出力 | `{base_path}/{character_name}/ai_processed/{filename}` |

### エラーハンドリング

- **個別スプライトの失敗**: ログに警告を出力し，スキップしてバッチ全体を継続
- **ヘルスチェック失敗**: バッチ全体を中止（`AppError::ComfyUI` を返却）
- **processing_jobs ステータス**:
  - `completed`: 全スプライト成功
  - `partial`: 一部成功
  - `failed`: 全スプライト失敗

### プログレスイベント

バッチ処理中は `comfyui-progress` イベントでフロントエンドに進捗を通知する．

```json
{
  "current": 3,
  "total": 10,
  "status": "processing",
  "current_file": "/path/to/input.png"
}
```

`status` は各スプライトの処理開始時に `"processing"`，完了時に `"completed"` が送信される．

---

## ポーリング仕様

ワークフロー実行の完了待ちはポーリング方式で行う（WebSocket は不使用）．

| パラメータ | 値 |
|-----------|-----|
| ポーリング間隔 | 1秒 |
| タイムアウト | 120秒 |

### ポーリングフロー

```
[開始]
  │
  ├─→ GET /history/{prompt_id}
  │   ├─ prompt_id のエントリが存在しない → 1秒待機して再試行
  │   ├─ status.completed == false        → 1秒待機して再試行
  │   ├─ status.completed == true かつ status_str == "error" → エラー返却
  │   └─ status.completed == true かつ status_str != "error" → 成功（結果返却）
  │
  └─ 120秒経過 → タイムアウトエラー
```

### 出力画像の特定

ポーリング完了後，`outputs` オブジェクトを走査し，最初に見つかった `images` 配列の先頭要素を結果画像として使用する．

```
history[prompt_id].outputs.{任意のnode_id}.images[0]
  → { filename, subfolder } を取得
  → GET /view で画像データをダウンロード
```

---

## トラブルシューティング

### ComfyUI に接続できない

| 確認項目 | 対処 |
|---------|------|
| ComfyUI が起動しているか | `http://127.0.0.1:8188/system_stats` にブラウザでアクセスして確認 |
| ポートが正しいか | ComfyUI のデフォルトポートは 8188．変更した場合はアプリ設定も更新 |
| ファイアウォール | ローカルホストの 8188 ポートがブロックされていないか確認 |
| CSP 設定 | エンドポイントを変更した場合，`src-tauri/tauri.conf.json` の CSP も更新が必要 |

### アップロードが失敗する

| 確認項目 | 対処 |
|---------|------|
| 画像ファイルが存在するか | 入力パス（`raw_path` / `processed_path`）のファイルが実在するか確認 |
| ComfyUI のディスク容量 | ComfyUI のワーキングディレクトリに十分な空き容量があるか確認 |
| ファイルパスに非ASCII文字 | パスに日本語等が含まれる場合，URL エンコーディングの問題が起きる可能性がある |

### 処理がタイムアウトする（120秒超過）

| 確認項目 | 対処 |
|---------|------|
| ComfyUI のキュー状態 | ComfyUI の Web UI でキューが詰まっていないか確認 |
| GPU メモリ | GPU の VRAM が不足していないか確認（タスクマネージャ等） |
| ワークフローの複雑さ | ステップ数やモデルサイズを見直し，処理時間を短縮 |
| ComfyUI がクラッシュ | ComfyUI のコンソールログでエラーを確認 |

### ワークフローの実行でエラーが返る

| 確認項目 | 対処 |
|---------|------|
| ワークフロー JSON の形式 | ComfyUI の「Save (API Format)」で出力した JSON を使用しているか確認 |
| プレースホルダの記法 | `{{key}}` の二重中括弧が正しいか確認（`{key}` は置換されない） |
| 必要なカスタムノード | ワークフローで使用しているカスタムノードが ComfyUI にインストール済みか確認 |
| モデルファイル | ワークフローが参照しているチェックポイント・LoRA が ComfyUI のモデルディレクトリに配置されているか確認 |

### 出力画像が見つからない

ワークフローの最終出力ノードが `SaveImage` または `PreviewImage` 等の画像出力ノードであることを確認する．出力ノードの `images` 配列が空の場合，結果画像が検出されずエラーとなる．
