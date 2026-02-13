# Figurine Studio -- DB 設計

## 概要

Figurine Studio は SQLite データベースを使用し，プロジェクト，キャラクター，スプライト，ワークフロー，処理ジョブ，アプリ設定を管理する．

**DB ファイル保存先:** `{app_data_dir}/figurine_studio.db`

`{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリである．

**接続設定（`src-tauri/src/db/mod.rs`）:**

```sql
PRAGMA journal_mode=WAL;     -- WAL モードで同時読み取りを許可
PRAGMA foreign_keys=ON;       -- 外部キー制約を有効化
PRAGMA busy_timeout=5000;     -- ロック待ちタイムアウト 5000ms
```

---

## ER 図（テキスト版）

```
┌──────────────────┐
│     projects     │
│──────────────────│
│ PK id            │
│    name          │
│    base_path     │
│    tile_width    │
│    tile_height   │
│    directions    │  ← JSON文字列
│    animations    │  ← JSON文字列
│    style_prompt  │
│    negative_     │
│      prompt      │
│    controlnet_   │
│      weight      │
│    bevy_output_  │
│      path        │
│    created_at    │
│    updated_at    │
└────────┬─────────┘
         │ 1
         │
         ├──────────────────────────────┐
         │ N                            │ N
┌────────▼─────────┐          ┌────────▼──────────────┐
│   characters     │          │  comfyui_workflows    │
│──────────────────│          │───────────────────────│
│ PK id            │          │ PK id                 │
│ FK project_id ───┤→projects │ FK project_id ────────┤→projects
│    name          │          │    name               │
│    category      │          │    workflow_json       │
│    custom_prompt │          │    is_default          │
│    status        │          │    created_at          │
│    spritesheet_  │          └───────────────────────┘
│      path        │
│    created_at    │
│    updated_at    │
└────────┬─────────┘
         │ 1
         │
         ├──────────────────────────────┐
         │ N                            │ N
┌────────▼─────────┐          ┌────────▼──────────────┐
│    sprites       │          │  processing_jobs      │
│──────────────────│          │───────────────────────│
│ PK id            │          │ PK id                 │
│ FK character_id──┤→chars    │ FK character_id ──────┤→characters
│    direction     │          │    job_type           │
│    animation     │          │    status             │
│    frame_index   │          │    prompt_id          │
│    raw_path      │          │    total_items        │
│    processed_    │          │    completed_items    │
│      path        │          │    error_message      │
│    final_path    │          │    created_at         │
│    status        │          │    updated_at         │
│    created_at    │          └───────────────────────┘
└──────────────────┘

┌──────────────────┐
│  app_settings    │
│──────────────────│
│ PK key           │
│    value         │
└──────────────────┘
```

**カスケード削除の関係:**
- `projects` 削除 -> `characters`, `comfyui_workflows` が連鎖削除
- `characters` 削除 -> `sprites`, `processing_jobs` が連鎖削除

---

## テーブル定義

### projects

プロジェクトの基本設定を保持する．方向（directions）とアニメーション定義（animations）は JSON 文字列として格納される．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `id` | TEXT | PRIMARY KEY | -- | UUID v4 |
| `name` | TEXT | NOT NULL | -- | プロジェクト名 |
| `base_path` | TEXT | NOT NULL | -- | ユーザー指定のベースディレクトリ |
| `tile_width` | INTEGER | NOT NULL | `64` | タイル幅（ピクセル） |
| `tile_height` | INTEGER | NOT NULL | `64` | タイル高（ピクセル） |
| `directions` | TEXT | NOT NULL | `'["down","left","right","up"]'` | 方向一覧（JSON 配列） |
| `animations` | TEXT | NOT NULL | *(後述)* | アニメーション定義（JSON 配列） |
| `style_prompt` | TEXT | NOT NULL | `'painted miniature figurine, soft PBR material, warm studio lighting, tabletop game piece'` | ComfyUI ポジティブプロンプト |
| `negative_prompt` | TEXT | NOT NULL | `'background, shadow on floor, realistic human'` | ComfyUI ネガティブプロンプト |
| `controlnet_weight` | REAL | NOT NULL | `0.8` | ControlNet ウェイト |
| `bevy_output_path` | TEXT | -- | NULL | Bevy エクスポート先パス |
| `created_at` | TEXT | NOT NULL | `datetime('now')` | 作成日時（ISO 8601） |
| `updated_at` | TEXT | NOT NULL | `datetime('now')` | 更新日時（ISO 8601） |

**`animations` のデフォルト値:**

```json
[
  {"name": "idle",   "frame_count": 2, "frame_duration_ms": 300},
  {"name": "walk",   "frame_count": 4, "frame_duration_ms": 150},
  {"name": "attack", "frame_count": 4, "frame_duration_ms": 100},
  {"name": "hit",    "frame_count": 2, "frame_duration_ms": 200}
]
```

**Rust モデル:** `src-tauri/src/models/project.rs` -- `Project`, `CreateProject`, `UpdateProject`
**クエリ:** `src-tauri/src/db/queries/project.rs`

---

### characters

プロジェクトに属するキャラクターを管理する．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `id` | TEXT | PRIMARY KEY | -- | UUID v4 |
| `project_id` | TEXT | NOT NULL, FK -> projects(id) ON DELETE CASCADE | -- | 所属プロジェクト |
| `name` | TEXT | NOT NULL | -- | キャラクター名 |
| `category` | TEXT | NOT NULL | `'enemy'` | カテゴリ（`player`, `enemy`, `npc`） |
| `custom_prompt` | TEXT | -- | NULL | キャラクター固有プロンプト |
| `status` | TEXT | NOT NULL | `'draft'` | 処理状態 |
| `spritesheet_path` | TEXT | -- | NULL | 生成済みスプライトシートのパス |
| `created_at` | TEXT | NOT NULL | `datetime('now')` | 作成日時 |
| `updated_at` | TEXT | NOT NULL | `datetime('now')` | 更新日時 |

**ステータス遷移:**

| status | 説明 |
|--------|------|
| `draft` | 初期状態・編集中 |
| `importing` | スプライトインポート中 |
| `processing` | AI処理・背景除去中 |
| `complete` | 全処理完了 |

**Rust モデル:** `src-tauri/src/models/character.rs` -- `Character`, `CreateCharacter`, `UpdateCharacter`
**クエリ:** `src-tauri/src/db/queries/character.rs`

---

### sprites

個々のスプライト画像とパイプライン上の各段階のファイルパスを管理する．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `id` | TEXT | PRIMARY KEY | -- | UUID v4 |
| `character_id` | TEXT | NOT NULL, FK -> characters(id) ON DELETE CASCADE | -- | 所属キャラクター |
| `direction` | TEXT | NOT NULL | -- | 方向名（例: `down`, `left`, `right`, `up`） |
| `animation` | TEXT | NOT NULL | -- | アニメーション名（例: `idle`, `walk`） |
| `frame_index` | INTEGER | NOT NULL | -- | フレーム番号（0始まり） |
| `raw_path` | TEXT | -- | NULL | インポートされた元画像のパス |
| `processed_path` | TEXT | -- | NULL | AI処理済み or 背景除去後の画像パス |
| `final_path` | TEXT | -- | NULL | 正規化後の最終画像パス |
| `status` | TEXT | NOT NULL | `'raw'` | スプライトの処理状態 |
| `created_at` | TEXT | NOT NULL | `datetime('now')` | 作成日時 |

**ステータス遷移:**

| status | 説明 | 該当パス |
|--------|------|---------|
| `raw` | インポート直後 | `raw_path` が設定済み |
| `ai_processed` | ComfyUI 質感変換完了 | `processed_path` が設定済み |
| `bg_removed` | 背景除去完了 | `processed_path` が更新済み |
| `finalized` | 正規化完了（スプライトシート用） | `final_path` が設定済み |

**パス優先順位ルール:**

各処理ステージは入力画像を `processed_path` > `raw_path` の優先順で選択する．これにより，途中のステージをスキップした場合でも前段の出力を自動的に利用できる．

**Rust モデル:** `src-tauri/src/models/sprite.rs` -- `Sprite`, `CreateSprite`, `UpdateSpritePaths`, `SpriteStatus`
**クエリ:** `src-tauri/src/db/queries/sprite.rs`

---

### comfyui_workflows

ComfyUI のワークフロー JSON を管理する．プロジェクトごとに複数のワークフローを保持でき，1つをデフォルトに設定できる．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `id` | TEXT | PRIMARY KEY | -- | UUID v4 |
| `project_id` | TEXT | NOT NULL, FK -> projects(id) ON DELETE CASCADE | -- | 所属プロジェクト |
| `name` | TEXT | NOT NULL | -- | ワークフロー名 |
| `workflow_json` | TEXT | NOT NULL | -- | ComfyUI ワークフロー JSON 文字列 |
| `is_default` | INTEGER | NOT NULL | `0` | デフォルトフラグ（0: false, 1: true） |
| `created_at` | TEXT | NOT NULL | `datetime('now')` | 作成日時 |

**デフォルトワークフロー管理:**
- `set_default_workflow` はトランザクション内で既存のデフォルトを解除してから新しいデフォルトを設定する
- プロジェクトあたり最大1つのデフォルトワークフローが存在する

**Rust モデル:** `src-tauri/src/models/workflow.rs` -- `Workflow`, `CreateWorkflow`
**クエリ:** `src-tauri/src/db/queries/workflow.rs`

---

### processing_jobs

長時間の非同期処理ジョブの状態を追跡する．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `id` | TEXT | PRIMARY KEY | -- | UUID v4 |
| `character_id` | TEXT | NOT NULL, FK -> characters(id) ON DELETE CASCADE | -- | 対象キャラクター |
| `job_type` | TEXT | NOT NULL | -- | ジョブ種別 |
| `status` | TEXT | NOT NULL | `'pending'` | ジョブ状態 |
| `prompt_id` | TEXT | -- | NULL | ComfyUI プロンプト ID |
| `total_items` | INTEGER | NOT NULL | `0` | 処理対象の総数 |
| `completed_items` | INTEGER | NOT NULL | `0` | 完了済みの数 |
| `error_message` | TEXT | -- | NULL | エラーメッセージ |
| `created_at` | TEXT | NOT NULL | `datetime('now')` | 作成日時 |
| `updated_at` | TEXT | NOT NULL | `datetime('now')` | 更新日時 |

**ジョブ種別:**

| job_type | 説明 |
|---------|------|
| `comfyui` | ComfyUI 質感変換バッチ |
| `bg_removal` | 背景除去バッチ |
| `normalize` | 正規化バッチ |
| `spritesheet` | スプライトシート生成 |

**ジョブ状態:**

| status | 説明 |
|--------|------|
| `pending` | 待機中 |
| `running` | 実行中 |
| `completed` | 全件成功 |
| `partial` | 一部成功（一部失敗あり） |
| `failed` | 全件失敗 |

---

### app_settings

キーバリュー形式のアプリケーション設定テーブル．

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `key` | TEXT | PRIMARY KEY | 設定キー |
| `value` | TEXT | NOT NULL | 設定値 |

**初期データ:**

| key | value | 説明 |
|-----|-------|------|
| `comfyui_endpoint` | `http://127.0.0.1:8188` | ComfyUI サーバーのエンドポイント |
| `onnx_model_path` | *(空文字列)* | ONNX モデルファイルのパス |
| `onnx_model_downloaded` | `false` | モデルダウンロード済みフラグ |
| `theme` | `dark` | UI テーマ（`dark` / `light`） |

**クエリ関数:**

| 関数 | 説明 |
|------|------|
| `get_setting(key)` | 単一設定値の取得（`Option<String>`） |
| `set_setting(key, value)` | 設定値の挿入または更新（`INSERT OR REPLACE`） |
| `get_all_settings()` | 全設定の `HashMap<String, String>` として取得 |

**Rust クエリ:** `src-tauri/src/db/queries/settings.rs`

---

### schema_version

マイグレーションの適用状況を追跡する内部管理テーブル．`db/schema.rs` の `run_migrations()` 関数により自動作成される．

| カラム | 型 | 制約 | デフォルト値 | 説明 |
|--------|-----|------|------------|------|
| `version` | INTEGER | NOT NULL | -- | マイグレーションバージョン番号 |
| `applied_at` | TEXT | NOT NULL | `datetime('now')` | 適用日時 |
| `description` | TEXT | -- | NULL | マイグレーションの説明 |

---

## インデックス一覧

| インデックス名 | テーブル | カラム | 用途 |
|---------------|---------|--------|------|
| `idx_characters_project` | `characters` | `project_id` | プロジェクト別キャラクター一覧の高速化 |
| `idx_sprites_character` | `sprites` | `character_id` | キャラクター別スプライト一覧の高速化 |
| `idx_sprites_status` | `sprites` | `status` | ステータス別スプライト検索の高速化 |
| `idx_processing_jobs_character` | `processing_jobs` | `character_id` | キャラクター別ジョブ一覧の高速化 |
| `idx_processing_jobs_status` | `processing_jobs` | `status` | ステータス別ジョブ検索の高速化 |

**参照ファイル:** `src-tauri/migrations/v1.sql`

---

## マイグレーションの仕組み

### 概要

`src-tauri/src/db/schema.rs` にバージョン管理式マイグレーションシステムが実装されている．

### 動作フロー

```
1. schema_version テーブルが存在しなければ作成
        ↓
2. schema_version から現在の最大バージョン番号を取得
        ↓
3. migrations() から全マイグレーション定義を取得
        ↓
4. 現在バージョンより大きいバージョンのマイグレーションを順次実行
        ↓
5. 実行したマイグレーションを schema_version に記録
```

### マイグレーション定義

```rust
struct Migration {
    version: i32,             // バージョン番号
    description: &'static str, // 説明文
    up: MigrationFn,          // 適用関数
}
```

**現在のマイグレーション一覧:**

| バージョン | 説明 | SQL ファイル |
|-----------|------|------------|
| v1 | Initial schema: projects, characters, sprites, workflows, jobs, settings | `src-tauri/migrations/v1.sql` |

### 新規マイグレーション追加手順

1. `src-tauri/migrations/v{N}.sql` に SQL を作成
2. `schema.rs` の `CURRENT_VERSION` を `N` に更新
3. `migrations()` 関数に新しい `Migration` エントリを追加
4. `migrate_v{N}` 関数を作成し，`include_str!` で SQL を読み込み実行

```rust
// schema.rs での追加例
const CURRENT_VERSION: i32 = 2;  // ← インクリメント

fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial schema",
            up: migrate_v1,
        },
        Migration {                     // ← 追加
            version: 2,
            description: "Add new table",
            up: migrate_v2,
        },
    ]
}

fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v2.sql"))?;
    Ok(())
}
```

**整合性チェック:**
- `run_migrations()` は起動時に `CURRENT_VERSION` と最後のマイグレーションバージョンが一致することを `assert` で検証する

**参照ファイル:** `src-tauri/src/db/schema.rs`, `src-tauri/src/db/mod.rs`

---

## クエリモジュール一覧

| モジュール | ファイル | 主要関数 |
|-----------|---------|---------|
| `project` | `src-tauri/src/db/queries/project.rs` | `create_project`, `get_project`, `list_projects`, `update_project`, `delete_project` |
| `character` | `src-tauri/src/db/queries/character.rs` | `create_character`, `get_character`, `list_characters_by_project`, `update_character`, `update_character_status`, `delete_character` |
| `sprite` | `src-tauri/src/db/queries/sprite.rs` | `create_sprite`, `create_sprites_batch`, `get_sprites_by_character`, `update_sprite_status`, `update_sprite_paths`, `update_sprite_assignment` |
| `workflow` | `src-tauri/src/db/queries/workflow.rs` | `create_workflow`, `get_workflow`, `list_all_workflows`, `list_workflows`, `update_workflow`, `delete_workflow`, `set_default_workflow`, `get_default_workflow` |
| `settings` | `src-tauri/src/db/queries/settings.rs` | `get_setting`, `set_setting`, `get_all_settings` |

### クエリパターン

**CRUD の基本パターン:**

```rust
pub fn create_entity(conn: &Connection, input: &CreateEntity) -> Result<Entity> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("INSERT INTO ...", params![id, ...])?;
    get_entity(conn, &id)?.ok_or_else(|| anyhow!("Failed to create"))
}
```

- ID 生成: UUID v4（`uuid::Uuid::new_v4().to_string()`）
- タイムスタンプ: RFC 3339 形式（`chrono::Utc::now().to_rfc3339()`）
- 作成後取得: `INSERT` 後に `SELECT` で作成されたレコードを返却
- バッチ処理: `unchecked_transaction()` でトランザクション化（`create_sprites_batch`）

**JSON フィールドの取り扱い:**

`projects.directions` と `projects.animations` は JSON 文字列として DB に格納される:

- 書き込み時: `serde_json::to_string()` で `Vec<String>` / `Vec<AnimationDef>` をシリアライズ
- 読み取り時: `serde_json::from_str()` でデシリアライズ（失敗時は空配列にフォールバック）

**参照ファイル:** `src-tauri/src/db/queries/` 配下の各ファイル
