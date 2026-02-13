# Figurine Studio -- アーキテクチャ全体図

## 概要

Figurine Studio は Tauri v2 ベースのデスクトップアプリケーションであり，ピクセルアートキャラクターのスプライトパイプラインを提供する．MagicaVoxel レンダリング画像のインポートから，AI テクスチャ生成（ComfyUI），背景除去（ONNX/U2-Net），スプライトシート合成，Bevy Engine 向けエクスポートまでを一貫して処理する．

## 2プロセスモデル

Tauri v2 のアーキテクチャに基づき，フロントエンド（Webview）とバックエンド（Rust プロセス）の2プロセスで構成される．

```
┌─────────────────────────────────────────────┐
│            Frontend (SvelteKit + Svelte 5)   │
│                                              │
│  ┌──────────┐  ┌─────────────┐  ┌────────┐  │
│  │  Routes   │→│  Stores      │→│  API    │  │
│  │ (+page)   │  │ (.svelte.ts) │  │ Layer  │  │
│  └──────────┘  └─────────────┘  └───┬────┘  │
│                                     │        │
│                     invoke()        │        │
│                     ─────────────── │ ──     │
│                     result/error    │        │
│                     ─────────────── │ ──     │
│                                     │        │
│         ┌───── Tauri Events ────────│───┐    │
│         │  (progress streaming)     │   │    │
│         ▼                           │   │    │
│  ┌──────────────┐                   │   │    │
│  │ Event         │                   │   │    │
│  │ Listeners     │                   │   │    │
│  └──────────────┘                   │   │    │
└─────────────────────────────────────│───│────┘
                                      │   │
              ┌───────────────────────▼───│────┐
              │         Backend (Rust)    │    │
              │                           │    │
              │  ┌────────────────────────▼─┐  │
              │  │  commands/                │  │
              │  │  (#[tauri::command])      │  │
              │  └────────┬─────────────────┘  │
              │           │                    │
              │  ┌────────▼────┐ ┌───────────┐ │
              │  │ db/queries/  │ │ services/  │ │
              │  │ (rusqlite)   │ │ (ONNX,    │ │
              │  │              │ │  ComfyUI,  │ │
              │  │              │ │  parser)   │ │
              │  └──────────────┘ └───────────┘ │
              │           │              │      │
              │  ┌────────▼──────────────▼───┐  │
              │  │      AppState              │  │
              │  │  - Mutex<Connection>       │  │
              │  │  - Mutex<Option<OnnxSvc>>  │  │
              │  │  - app_data_dir            │  │
              │  │  - models_dir              │  │
              │  └────────────────────────────┘  │
              └──────────────────────────────────┘
```

### 通信方式

| 方向 | 方式 | 用途 |
|------|------|------|
| Frontend -> Backend | `invoke()` | リクエスト/レスポンス型のコマンド呼び出し |
| Backend -> Frontend | Tauri Events (`emit()`) | 長時間処理の進捗ストリーミング |

**参照ファイル:** `src/lib/api/tauri.ts`, `src-tauri/src/lib.rs`

---

## フロントエンド構成

### ルーティング

SvelteKit のファイルベースルーティングを採用し，`adapter-static` による SPA として配信される．

```
src/routes/
├── +layout.svelte                      # グローバルレイアウト
├── +layout.ts                          # SSR無効化設定
├── +page.svelte                        # トップ（プロジェクト一覧）
├── project/
│   ├── new/+page.svelte                # プロジェクト新規作成
│   └── [id]/
│       ├── +layout.svelte              # プロジェクト詳細レイアウト
│       └── +page.svelte                # プロジェクト詳細・キャラクター一覧
├── character/
│   ├── new/+page.svelte                # キャラクター新規作成
│   └── [id]/
│       ├── +layout.svelte              # キャラクタータブレイアウト
│       ├── +page.svelte                # キャラクター概要
│       ├── import/+page.svelte         # スプライトインポート
│       ├── process/+page.svelte        # AI処理・背景除去
│       └── preview/+page.svelte        # プレビュー・エクスポート
└── settings/
    ├── +page.svelte                    # 設定トップ
    ├── models/+page.svelte             # ONNXモデル管理
    └── comfyui/+page.svelte            # ComfyUI接続設定
```

**参照ファイル:** `src/routes/` 配下の各ファイル

### 状態管理

Svelte 5 の runes（`$state`, `$derived`）を使用したストアパターンを採用している．外部状態管理ライブラリは不使用で，各ストアがモジュールレベルの `$state` 変数とシングルトンオブジェクトをエクスポートする．

```
src/lib/stores/
├── project.svelte.ts       # プロジェクト状態
├── character.svelte.ts     # キャラクター状態
├── sprite.svelte.ts        # スプライト状態
├── onnx.svelte.ts          # ONNXモデル状態
├── comfyui.svelte.ts       # ComfyUI接続状態
├── settings.svelte.ts      # アプリ設定
├── setup.svelte.ts         # 初期セットアップ状態
└── toast.svelte.ts         # トースト通知
```

**ストアの構成パターン:**

```typescript
// モジュールレベルの$state変数
let items = $state<Item[]>([]);
let loading = $state(false);

// シングルトンオブジェクトのエクスポート
export const store = {
  get items() { return items; },       // リアクティブgetter
  get loading() { return loading; },
  async load() { ... },                // 非同期アクション
};
```

**参照ファイル:** `src/lib/stores/*.svelte.ts`

### API レイヤー

`src/lib/api/tauri.ts` がフロントエンドからバックエンドへの全通信を集約する．ドメインごとにグループ化された API オブジェクトを提供する．

| API オブジェクト | 担当ドメイン | 主要メソッド |
|-----------------|-------------|-------------|
| `projectApi` | プロジェクト CRUD | `create`, `get`, `list`, `update`, `delete` |
| `characterApi` | キャラクター CRUD + インポート | `create`, `get`, `list`, `update`, `delete`, `importSprites` |
| `spriteApi` | スプライト操作 | `list`, `updateAssignment`, `normalizeBatch`, `generateSpritesheet`, `exportForBevy` |
| `onnxApi` | ONNX モデル管理 | `checkModel`, `downloadModel`, `initialize`, `removeBackground`, `removeBackgroundBatch` |
| `comfyuiApi` | ComfyUI 連携 | `checkConnection`, `listWorkflows`, `createWorkflow`, `processBatch` |
| `settingsApi` | アプリ設定 | `getAll`, `update` |

**イベントリスナーヘルパー:**

| 関数 | イベント名 | ペイロード型 |
|------|-----------|------------|
| `onDownloadProgress` | `download-progress` | `DownloadProgress` |
| `onProcessingProgress` | `processing-progress` | `ProcessingProgress` |
| `onComfyUIProgress` | `comfyui-progress` | `ComfyUIProgress` |
| `onBgRemovalProgress` | `bg-removal-progress` | `BgRemovalProgress` |

**参照ファイル:** `src/lib/api/tauri.ts`

### 型定義

`src/lib/types/index.ts` に全 TypeScript 型定義が集約されている．バックエンド Rust モデル（`src-tauri/src/models/`）と同期を維持する必要がある．

主要な型:

- **エンティティ:** `Project`, `Character`, `Sprite`, `Workflow`, `ProcessingJob`
- **作成/更新:** `CreateProject`, `UpdateProject`, `CreateCharacter`, `UpdateCharacter`, `CreateWorkflow`
- **列挙型:** `SpriteStatus`, `CharacterStatus`, `CharacterCategory`, `OnnxModelState`, `ComfyUIConnectionStatus`
- **処理結果:** `SpritesheetResult`, `BevyExportResult`, `ExportResult`
- **イベントペイロード:** `DownloadProgress`, `ProcessingProgress`, `ComfyUIProgress`, `BgRemovalProgress`
- **パラメータ:** `ProcessingParams`, `AnimationDef`, `DirectionDef`

**参照ファイル:** `src/lib/types/index.ts`, `src-tauri/src/models/`

---

## バックエンド構成

### エントリポイント

`src-tauri/src/lib.rs` がアプリケーションのエントリポイントであり，以下の初期化を行う:

1. `env_logger` の初期化
2. Tauri プラグインの登録（`dialog`, `fs`, `shell`）
3. `app_data_dir` の取得と `AppState` の生成
4. 全コマンドハンドラの登録（`generate_handler![]`）

**登録コマンド一覧（11モジュール，24コマンド）:**

| モジュール | コマンド数 | 主要コマンド |
|-----------|-----------|------------|
| `project_commands` | 5 | `create_project`, `list_projects`, `delete_project` |
| `character_commands` | 6 | `create_character`, `import_sprites` |
| `image_processing` | 2 | `normalize_sprite`, `normalize_batch` |
| `model_download` | 2 | `check_onnx_model`, `download_onnx_model` |
| `background_removal` | 3 | `initialize_onnx`, `remove_background_batch` |
| `spritesheet` | 1 | `generate_spritesheet` |
| `bevy_export` | 1 | `export_for_bevy` |
| `comfyui` | 7 | `process_batch_comfyui`, `import_workflow` |
| `settings_commands` | 2 | `get_settings`, `update_setting` |
| `sprite_commands` | 2 | `list_sprites`, `update_sprite_assignment` |

**参照ファイル:** `src-tauri/src/lib.rs`

### AppState

`src-tauri/src/state.rs` で定義されるアプリケーション共有状態．全コマンドが `tauri::State<'_, AppState>` 経由でアクセスする．

```rust
pub struct AppState {
    pub db: Mutex<Connection>,              // SQLite接続（WALモード）
    pub onnx_service: Mutex<Option<OnnxService>>,  // 遅延初期化ONNX
    pub app_data_dir: PathBuf,              // アプリデータディレクトリ
    pub models_dir: PathBuf,                // ONNXモデル格納先
}
```

- `db`: rusqlite の `Connection` を `Mutex` で保護．WAL モードで同時読み取りを許可
- `onnx_service`: U2-Net モデルの遅延ロード．ユーザーがモデルダウンロード後に `initialize_onnx` で初期化
- `app_data_dir`: `{app_data_dir}` -- Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリ
- `models_dir`: `{app_data_dir}/models/` -- ONNX モデルファイルの格納先

**参照ファイル:** `src-tauri/src/state.rs`

### コマンドパターン

全コマンドは以下の統一パターンに従う:

```rust
#[tauri::command]
pub async fn command_name(
    state: tauri::State<'_, AppState>,
    // ... パラメータ
) -> Result<T, AppError> {
    // 1. state.db.lock() で DB 接続を取得
    // 2. バリデーション
    // 3. db::queries::* でクエリ実行
    // 4. Result<T, AppError> を返却
}
```

**参照ファイル:** `src-tauri/src/commands/` 配下の各ファイル

### サービス層

```
src-tauri/src/services/
├── mod.rs
├── onnx_service.rs       # U2-Net背景除去エンジン
├── comfyui_client.rs     # ComfyUI HTTP/RESTクライアント
├── filename_parser.rs    # MagicaVoxelファイル名パーサー
└── path_utils.rs         # パスサニタイズ・検証ユーティリティ
```

| サービス | 責務 | 外部依存 |
|---------|------|---------|
| `OnnxService` | U2-Net による背景除去推論 | ort (ONNX Runtime) |
| `ComfyUIClient` | ComfyUI REST API との通信 | reqwest (HTTP) |
| `FilenameParser` | `{character}_{direction}_{animation}_{frame}.png` 形式の解析 | なし |
| `path_utils` | ファイル名サニタイズ，パストラバーサル検証，ディレクトリ作成 | なし |

**参照ファイル:** `src-tauri/src/services/`

### エラー処理

`src-tauri/src/error.rs` で定義される `AppError` 列挙型が，バックエンド全体のエラーを統一的に管理する．

```rust
pub enum AppError {
    NotFound(String),           // リソース未発見（404相当）
    Validation(String),         // 入力バリデーションエラー
    Database(String),           // DB操作エラー
    Io(String),                 // ファイルI/Oエラー
    Onnx(String),               // ONNXランタイムエラー
    ComfyUI(String),            // ComfyUI連携エラー
    ImageProcessing(String),    // 画像処理エラー
    Export(String),             // エクスポートエラー
    Internal(String),           // 予期しない内部エラー
}
```

**設計方針:**

- `thiserror` によるエラーメッセージ生成（日本語のユーザー向けメッセージ）
- `Serialize` を実装し Tauri コマンドの戻り値としてそのまま返却可能
- `error_code()` メソッドでフロントエンド側のエラー種別分岐に使用するコード文字列を返却
- `From` トレイトで `anyhow::Error`, `rusqlite::Error`, `std::io::Error`, `serde_json::Error` からの自動変換

| エラーコード | 対応バリアント | 用途 |
|-------------|--------------|------|
| `NOT_FOUND` | `NotFound` | リソースが見つからない |
| `VALIDATION` | `Validation` | 入力値の不正 |
| `DATABASE` | `Database` | SQLite 操作失敗 |
| `IO` | `Io` | ファイル読み書きの失敗 |
| `ONNX` | `Onnx` | ONNX 推論・モデルロードの失敗 |
| `COMFYUI` | `ComfyUI` | ComfyUI 接続・処理の失敗 |
| `IMAGE_PROCESSING` | `ImageProcessing` | 画像変換処理の失敗 |
| `EXPORT` | `Export` | エクスポート処理の失敗 |
| `INTERNAL` | `Internal` | 予期しないエラー |

**参照ファイル:** `src-tauri/src/error.rs`

---

## セキュリティ

### CSP（Content Security Policy）

`src-tauri/tauri.conf.json` で以下の CSP が設定されている:

```
default-src 'self';
img-src 'self' asset: https://asset.localhost;
connect-src 'self' http://127.0.0.1:8188 ws://127.0.0.1:8188;
style-src 'self' 'unsafe-inline'
```

| ディレクティブ | 設定 | 理由 |
|--------------|------|------|
| `default-src` | `'self'` | Tauri アプリ内リソースのみ許可 |
| `img-src` | `'self' asset: https://asset.localhost` | ローカル画像ファイルの表示用 |
| `connect-src` | `'self' http://127.0.0.1:8188 ws://127.0.0.1:8188` | ローカル ComfyUI サーバーへの HTTP/WebSocket 接続許可 |
| `style-src` | `'self' 'unsafe-inline'` | インラインスタイルの許可 |

### Tauri プラグインスコープ

```json
{
  "plugins": {
    "dialog": {},
    "fs": { "scope": ["**"] },
    "shell": { "open": true }
  }
}
```

- `fs`: 全パスへのアクセスを許可（スプライト画像の読み書きに必要）
- `shell`: URL オープンのみ許可（外部コマンド実行は不可）
- `dialog`: ファイル選択ダイアログの利用

### パストラバーサル防止

`src-tauri/src/services/path_utils.rs` の `validate_user_path()` で以下を検証する:

- 絶対パスの拒否（ユーザー入力はベースパスからの相対パスであるべき）
- `..` セグメントの拒否（ディレクトリトラバーサル攻撃の防止）

また `sanitize_filename()` で安全なファイル名生成を行う:

- 英数字・アンダースコア・ハイフン以外の文字をアンダースコアに置換
- 小文字に正規化
- 全文字が記号のみの場合はエラー

**参照ファイル:** `src-tauri/src/services/path_utils.rs`, `src-tauri/tauri.conf.json`

---

## ウィンドウ設定

| 項目 | 値 |
|------|-----|
| タイトル | Figurine Studio |
| 初期サイズ | 1280 x 800 |
| 最小サイズ | 1024 x 768 |

**参照ファイル:** `src-tauri/tauri.conf.json`

---

## 技術スタック

### フロントエンド

| 技術 | バージョン/詳細 |
|------|---------------|
| SvelteKit | adapter-static（SPA） |
| Svelte | 5（runes ベース状態管理） |
| Tauri API | `@tauri-apps/api` |
| ビルドツール | Vite (dev port: 5173) |

### バックエンド

| 技術 | 用途 |
|------|------|
| Tauri v2 | デスクトップアプリフレームワーク |
| rusqlite | SQLite データベース（WAL モード） |
| ort | ONNX Runtime バインディング |
| reqwest | HTTP クライアント（ComfyUI 通信） |
| image | 画像処理（リサイズ，クロップ，合成） |
| serde / serde_json | シリアライゼーション |
| thiserror | エラー型定義 |
| uuid | UUID v4 生成 |
| chrono | タイムスタンプ生成 |
| env_logger / log | ロギング |

### UI 言語

アプリケーションの UI テキスト（トーストメッセージ，エラー表示，バリデーションメッセージ）は日本語で記述されている．
