# システム起動シーケンス

Figurine Studio の起動からメイン UI 表示までの全フローを解説する．

## 概要

Figurine Studio は Tauri v2 の Two-Process Model で動作する．起動時は以下の 5 フェーズを順に実行する．

```
Phase 1: 開発モード起動 (cargo tauri dev)
    │
    ├─→ Vite dev server 起動 (localhost:5173)
    │
    └─→ Phase 2: Rust バックエンド初期化
            │
            ├─ env_logger 初期化
            ├─ Tauri plugins 登録
            ├─ setup hook 実行
            │   ├─ app_data_dir 取得・作成
            │   ├─ AppState::new()
            │   │   ├─ models_dir 作成
            │   │   ├─ SQLite 接続 (WAL モード)
            │   │   └─ DB マイグレーション実行
            │   └─ app.manage(app_state)
            │
            └─→ Webview ウィンドウ表示
                    │
                    └─→ Phase 3: フロントエンド初期化
                            │
                            ├─ settingsStore.loadSettings()
                            │   └─ invoke('get_settings')
                            │       → theme / bevy_export_path / comfyui_endpoint 取得
                            │
                            └─ setupStore.checkSetupRequired()
                                ├─ invoke('get_settings') → setup_completed 確認
                                │
                                ├─[setup_completed === 'true']
                                │   └─→ Phase 5: 通常起動
                                │
                                └─[setup_completed !== 'true']
                                    └─→ Phase 4: 初回セットアップ
                                            │
                                            ├─ Step 1: ONNX モデルダウンロード
                                            │   └─ U2-Net モデル (~176MB)
                                            │
                                            ├─ Step 2: ComfyUI 接続設定
                                            │   └─ (スキップ可能)
                                            │
                                            └─→ Phase 5: 通常起動
                                                    └─ SetupWizard 非表示
                                                        → メイン UI 表示
```

---

## Phase 1: 開発モード起動

### コマンド

```bash
cargo tauri dev
```

### 実行フロー

1. Tauri CLI が `tauri.conf.json` の `build.beforeDevCommand` を参照する
2. `pnpm dev`（= `vite dev`）がフロントエンドの開発サーバーを起動する
3. Vite が `localhost:5173` でホットリロード対応の SvelteKit アプリを配信開始する
4. 並行して Rust バックエンド（`src-tauri/`）がコンパイル・起動される
5. Tauri の Webview が `build.devUrl`（`http://localhost:5173`）に接続する

### 関連設定

`tauri.conf.json` の該当箇所:

```json
{
  "build": {
    "frontendDist": "../build",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  }
}
```

`package.json` のスクリプト:

```json
{
  "scripts": {
    "dev": "vite dev",
    "build": "vite build"
  }
}
```

---

## Phase 2: Rust バックエンド初期化

Rust 側のエントリポイントは `src-tauri/src/main.rs` → `src-tauri/src/lib.rs` の `run()` 関数である．

### 2.1 env_logger 初期化

```rust
// src-tauri/src/lib.rs
env_logger::init();
```

環境変数 `RUST_LOG` に基づいてログレベルを設定する．開発時は `RUST_LOG=info cargo tauri dev` のようにログレベルを制御できる．

### 2.2 Tauri Builder 構築と Plugins 登録

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())   // ファイルダイアログ
    .plugin(tauri_plugin_fs::init())       // ファイルシステムアクセス
    .plugin(tauri_plugin_shell::init())    // 外部プロセス起動 / URL オープン
```

各プラグインは `tauri.conf.json` の `plugins` セクションでスコープ設定される:

| プラグイン | 用途 | スコープ |
|:---|:---|:---|
| `tauri-plugin-dialog` | ファイル選択ダイアログ | 制限なし |
| `tauri-plugin-fs` | ファイル読み書き | `"scope": ["**"]`（全パスアクセス可） |
| `tauri-plugin-shell` | URL オープン | `"open": true` |

### 2.3 setup hook

`setup` クロージャ内で `AppState` の生成と Tauri への登録を行う．

```rust
.setup(|app| {
    // 2.3.1 app_data_dir の取得と作成
    let app_data_dir = app.path().app_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&app_data_dir)?;

    // 2.3.2 AppState の生成
    let app_state = AppState::new(app_data_dir)?;

    // 2.3.3 Tauri State として登録
    app.manage(app_state);
    Ok(())
})
```

#### 2.3.1 `app_data_dir` について

`{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリである．アプリの識別子 `com.figurine-studio.app`（`tauri.conf.json` の `identifier`）に基づいて OS が決定するパスに，SQLite データベースや ONNX モデルなどの永続データが格納される．

#### 2.3.2 AppState の生成

`AppState::new()` は以下の処理を順に実行する:

```rust
// src-tauri/src/state.rs
pub fn new(app_data_dir: PathBuf) -> anyhow::Result<Self> {
    // (a) models_dir の作成
    let models_dir = app_data_dir.join("models");
    std::fs::create_dir_all(&models_dir)?;

    // (b) SQLite データベース接続
    let db_path = app_data_dir.join("figurine_studio.db");
    let conn = Connection::open(&db_path)?;

    // (c) DB 初期化（PRAGMA 設定 + マイグレーション）
    crate::db::initialize(&conn)?;

    Ok(Self {
        db: Mutex::new(conn),
        onnx_service: Mutex::new(None),  // 遅延初期化
        app_data_dir,
        models_dir,
    })
}
```

`AppState` の構成:

| フィールド | 型 | 説明 |
|:---|:---|:---|
| `db` | `Mutex<Connection>` | SQLite 接続（排他ロック） |
| `onnx_service` | `Mutex<Option<OnnxService>>` | ONNX Runtime（遅延初期化，初期値 `None`） |
| `app_data_dir` | `PathBuf` | アプリデータディレクトリのパス |
| `models_dir` | `PathBuf` | `{app_data_dir}/models` |

#### 2.3.3 DB 初期化

`db::initialize()` が SQLite の PRAGMA 設定とマイグレーションを実行する．

```rust
// src-tauri/src/db/mod.rs
pub fn initialize(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")?;

    schema::run_migrations(conn)?;
    Ok(())
}
```

**PRAGMA 設定:**

| PRAGMA | 値 | 目的 |
|:---|:---|:---|
| `journal_mode` | `WAL` | 書き込み中の読み取りを許可（パフォーマンス向上） |
| `foreign_keys` | `ON` | 外部キー制約を有効化（CASCADE DELETE に必須） |
| `busy_timeout` | `5000` | ロック競合時に最大 5 秒待機 |

**マイグレーション:**

`schema::run_migrations()` はバージョン管理方式でスキーマを適用する:

1. `schema_version` テーブルが存在しなければ作成する
2. 現在適用済みの最大バージョンを取得する
3. 未適用のマイグレーションを順に実行する
4. 各マイグレーション適用後にバージョンレコードを挿入する

初回起動時は v1 マイグレーション（`src-tauri/migrations/v1.sql`）が実行され，以下のテーブルが作成される:

- `projects` ― プロジェクト管理
- `characters` ― キャラクター管理
- `sprites` ― スプライト画像管理
- `comfyui_workflows` ― ComfyUI ワークフロー定義
- `processing_jobs` ― 処理ジョブ追跡
- `app_settings` ― アプリケーション設定（Key-Value）

`app_settings` テーブルには初期値として以下が挿入される:

| key | value |
|:---|:---|
| `comfyui_endpoint` | `http://127.0.0.1:8188` |
| `onnx_model_path` | （空文字列） |
| `onnx_model_downloaded` | `false` |
| `theme` | `dark` |

### 2.4 コマンドハンドラの登録

`invoke_handler` マクロで全ての Tauri コマンドを登録する．フロントエンドから `invoke()` で呼び出し可能なコマンド群は以下のドメインに分類される:

- **Project** ― CRUD 操作
- **Character** ― CRUD 操作 + スプライトインポート
- **Image Processing** ― 正規化処理
- **Model Download** ― ONNX モデルの確認・ダウンロード
- **Background Removal** ― ONNX 初期化・背景除去
- **Spritesheet** ― スプライトシート生成
- **Bevy Export** ― Bevy Engine 向けエクスポート
- **ComfyUI** ― 接続確認・ワークフロー管理・バッチ処理
- **Settings** ― 設定の取得・更新
- **Sprite** ― スプライト一覧・割り当て更新

### 2.5 アプリケーション起動

```rust
.run(tauri::generate_context!())
.expect("error while running tauri application");
```

`tauri::generate_context!()` が `tauri.conf.json` を埋め込み，Webview ウィンドウを生成する．ウィンドウ設定:

| 項目 | 値 |
|:---|:---|
| タイトル | `Figurine Studio` |
| 初期サイズ | 1280 x 800 |
| 最小サイズ | 1024 x 768 |

CSP（Content Security Policy）により，ComfyUI（`127.0.0.1:8188`）への HTTP/WebSocket 接続が許可される:

```
connect-src 'self' http://127.0.0.1:8188 ws://127.0.0.1:8188
```

---

## Phase 3: フロントエンド初期化

Webview がフロントエンドを読み込むと，SvelteKit のルートレイアウト（`src/routes/+layout.svelte`）が最初に実行される．

### 3.1 settingsStore.loadSettings()

`$effect()` により即座に実行される．

```typescript
// src/routes/+layout.svelte
$effect(() => {
    settingsStore.loadSettings();
});
```

**処理内容:**

1. `invoke('get_settings')` で `app_settings` テーブルの全レコードを取得する
2. `theme`（`'light'` or `'dark'`）を適用する（`document.documentElement.setAttribute('data-theme', ...)`）
3. `bevy_export_path`，`comfyui_endpoint` をストアに保持する
4. 取得失敗時はトースト通知で「設定の読み込みに失敗しました」と表示する

### 3.2 setupStore.checkSetupRequired()

同じく `$effect()` で実行される．

```typescript
// src/routes/+layout.svelte
$effect(() => {
    setupStore.checkSetupRequired();
});
```

**処理内容:**

1. `invoke('get_settings')` で全設定を取得する
2. `setup_completed` キーの値を確認する
   - `'true'` の場合 → `setupRequired = false` とし，Phase 5（通常起動）に進む
   - それ以外の場合 → ONNX モデルの状態を確認する
3. ONNX モデルが既にダウンロード済みなら `currentStep = 2`（ComfyUI 設定から開始）にスキップする
4. `setupRequired = true` として Phase 4（初回セットアップ）を開始する
5. エラー発生時はセットアップをスキップしてアプリの動作を妨げない

---

## Phase 4: 初回セットアップ（SetupWizard）

`setupRequired === true` の場合，`SetupWizard` コンポーネントがオーバーレイとして表示される．

### 全体フロー

```
SetupWizard (オーバーレイ表示)
│
├── Step 1: ONNX モデルのセットアップ
│   ├── ModelDownloader コンポーネント
│   │   ├── check_onnx_model → モデル存在確認
│   │   │   └── {app_data_dir}/models/u2net.onnx を確認
│   │   ├── [未ダウンロード] → download_onnx_model
│   │   │   ├── GitHub Releases からストリーミング DL
│   │   │   ├── 一時ファイル (.tmp) に書き込み
│   │   │   ├── download-progress イベントで進捗通知
│   │   │   └── アトミックリネームで完了
│   │   └── [ダウンロード済み] → 自動で Step 2 へ
│   │
│   └── [完了] → nextStep() → Step 2 へ
│
├── Step 2: ComfyUI 接続設定
│   ├── ComfyUIGuide コンポーネント
│   │   ├── loadEndpoint() → DB からエンドポイント読み込み
│   │   ├── デフォルト: http://127.0.0.1:8188
│   │   └── checkConnection() → 接続テスト
│   │
│   ├── [接続成功] → completeSetup()
│   └── [スキップ] → skipComfyUI() → completeSetup()
│
└── completeSetup()
    ├── app_settings に 'setup_completed' = 'true' を書き込み
    └── setupRequired = false → SetupWizard 非表示
```

### Step 1: ONNX モデルダウンロード

背景除去に使用する U2-Net モデル（約 176MB）をダウンロードする．

**バックエンド側の処理（`commands/model_download.rs`）:**

1. `check_onnx_model` コマンド: `{app_data_dir}/models/u2net.onnx` の存在とサイズを確認する
2. `download_onnx_model` コマンド:
   - ダウンロード元: `https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx`
   - 一時ファイル `u2net.onnx.tmp` にストリーミング書き込みする
   - `download-progress` イベントでフロントエンドにバイト単位の進捗を通知する
   - ファイルサイズ検証後，アトミックリネーム（`.tmp` → `.onnx`）で完了する
   - `app_settings` テーブルの `onnx_model_downloaded` を `'true'` に更新する

**フロントエンド側の状態遷移（`onnxStore`）:**

```
not_downloaded → downloading → downloaded → initializing → ready
                     ↓                            ↓
                   error                        error
```

### Step 2: ComfyUI 接続設定

ComfyUI は AI テクスチャ生成に使用する外部サービスである．**このステップはスキップ可能**であり，背景除去とスプライトシート生成は ComfyUI なしでも動作する．

- エンドポイントのデフォルト値: `http://127.0.0.1:8188`
- `checkConnection()` で HTTP 接続テストを行う
- 接続成功時，エンドポイントを `app_settings` に永続化する
- 「ComfyUI をスキップ」ボタンで Step 2 を省略できる

### セットアップ完了

`completeSetup()` が呼ばれると:

1. `invoke('update_setting', { key: 'setup_completed', value: 'true' })` で DB に記録する
2. `setupRequired = false` となりオーバーレイが非表示になる
3. 次回起動時は Phase 3 の `checkSetupRequired()` で `setup_completed === 'true'` が検出され，Phase 5 に直行する

**注意:** 「全てスキップ」ボタンは `setup_completed` を DB に書き込まない．そのため次回起動時にセットアップウィザードが再表示される．

---

## Phase 5: 通常起動

`setupRequired === false` の状態では `SetupWizard` コンポーネントが描画されず，メイン UI がそのまま表示される．

### メイン UI の構成

```
+------------------------------------------+
|  Sidebar (240px / 60px collapsed)        |
|  +---------+----------------------------+|
|  | Sidebar | main-content              ||
|  |         | (SvelteKit ルーティング)    ||
|  |         |                            ||
|  +---------+----------------------------+|
|  Toast 通知 (右下)                        |
+------------------------------------------+
```

- **Sidebar**: プロジェクト一覧，ナビゲーション
- **main-content**: `{@render children()}` によるファイルベースルーティング
- **Toast**: 操作結果の通知表示

---

## 本番ビルドとの違い

### ビルドコマンド

```bash
cargo tauri build
```

### 開発モードと本番ビルドの比較

| 項目 | 開発モード (`cargo tauri dev`) | 本番ビルド (`cargo tauri build`) |
|:---|:---|:---|
| フロントエンド配信 | Vite dev server (`localhost:5173`) | 静的ビルド成果物（`build/` ディレクトリ） |
| ビルド前コマンド | `pnpm dev` | `pnpm build`（= `vite build`） |
| フロントエンド読み込み元 | `build.devUrl` | `build.frontendDist` |
| HMR（ホットリロード） | 有効 | なし |
| Rust コンパイル | デバッグビルド | リリースビルド（最適化あり） |
| `windows_subsystem` | 通常コンソール | `"windows"`（Windows でコンソール非表示） |
| DevTools | 利用可能 | 無効 |
| バイナリサイズ | 大（デバッグ情報含む） | 小（最適化・ストリップ済み） |

### 本番ビルド時のフロントエンド

`beforeBuildCommand` として `pnpm build` が実行され，SvelteKit の `adapter-static` が `build/` ディレクトリに静的ファイルを生成する．Tauri はこの静的ファイルをバイナリに埋め込み，Webview から直接読み込む．ネットワークサーバーは不要である．

### Rust 側の分岐

`src-tauri/src/main.rs` の `#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` により，リリースビルドでは Windows 上でコンソールウィンドウが表示されない．Phase 2 以降の初期化フローは開発・本番で同一である．

---

## ディレクトリ構造（起動時に参照されるファイル）

```
src-tauri/
├── src/
│   ├── main.rs              # Rust エントリポイント
│   ├── lib.rs               # run(): Tauri Builder 構築
│   ├── state.rs             # AppState: DB + ONNX + パス管理
│   ├── db/
│   │   ├── mod.rs           # initialize(): PRAGMA + マイグレーション
│   │   └── schema.rs        # run_migrations(): バージョン管理
│   └── commands/
│       ├── model_download.rs    # ONNX モデル確認・DL
│       └── settings_commands.rs # 設定取得・更新
├── migrations/
│   └── v1.sql               # 初期スキーマ定義
└── tauri.conf.json           # Tauri 設定（ウィンドウ・CSP・プラグイン）

src/
├── routes/
│   └── +layout.svelte       # ルートレイアウト（初期化起点）
└── lib/
    ├── api/
    │   └── tauri.ts          # invoke() ラッパー
    ├── stores/
    │   ├── settings.svelte.ts # settingsStore: 設定読み込み
    │   ├── setup.svelte.ts    # setupStore: セットアップ判定
    │   ├── onnx.svelte.ts     # onnxStore: モデル管理
    │   └── comfyui.svelte.ts  # comfyuiStore: ComfyUI 接続
    └── components/
        └── SetupWizard.svelte # セットアップウィザード UI
```
