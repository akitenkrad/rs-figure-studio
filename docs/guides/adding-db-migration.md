# DB マイグレーション追加手順

Figurine Studio の SQLite データベーススキーマを変更する手順を説明する．

## 概要

本プロジェクトでは rusqlite を使用した独自のマイグレーションシステムを採用している．マイグレーションはバージョン管理されたSQLファイルとして管理され，アプリ起動時に自動適用される．

スキーマ変更は以下の 3 ステップで行う：

1. **SQL ファイル作成** — `src-tauri/migrations/` に新しいバージョンの SQL を追加
2. **schema.rs 更新** — `src-tauri/src/db/schema.rs` にマイグレーション関数を登録
3. **クエリ・モデル・型の同期** — Rust モデル，DB クエリ，TypeScript 型を更新

## 現在のスキーマ構成

### テーブル一覧（v1）

`src-tauri/migrations/v1.sql` で定義されている初期スキーマ：

| テーブル | 用途 | 主要カラム |
|---------|------|-----------|
| `projects` | プロジェクト管理 | `id`, `name`, `base_path`, `tile_width`, `tile_height`, `directions` (JSON), `animations` (JSON), `style_prompt`, `negative_prompt`, `controlnet_weight` |
| `characters` | キャラクター管理 | `id`, `project_id` (FK), `name`, `category`, `custom_prompt`, `status`, `spritesheet_path` |
| `sprites` | スプライト画像管理 | `id`, `character_id` (FK), `direction`, `animation`, `frame_index`, `raw_path`, `processed_path`, `final_path`, `status` |
| `comfyui_workflows` | ComfyUI ワークフロー | `id`, `project_id` (FK), `name`, `workflow_json`, `is_default` |
| `processing_jobs` | 処理ジョブ管理 | `id`, `character_id` (FK), `job_type`, `status`, `prompt_id`, `total_items`, `completed_items`, `error_message` |
| `app_settings` | アプリ設定 (KV) | `key`, `value` |
| `schema_version` | マイグレーション履歴 | `version`, `applied_at`, `description` |

### 外部キー・カスケード関係

```
projects ──┬── characters ──── sprites
           │       └──────── processing_jobs
           └── comfyui_workflows
```

- `characters.project_id` → `projects.id` (`ON DELETE CASCADE`)
- `sprites.character_id` → `characters.id` (`ON DELETE CASCADE`)
- `comfyui_workflows.project_id` → `projects.id` (`ON DELETE CASCADE`)
- `processing_jobs.character_id` → `characters.id` (`ON DELETE CASCADE`)

### インデックス

```sql
idx_characters_project      -- characters(project_id)
idx_sprites_character        -- sprites(character_id)
idx_sprites_status           -- sprites(status)
idx_processing_jobs_character -- processing_jobs(character_id)
idx_processing_jobs_status   -- processing_jobs(status)
```

## ステップ 1: SQL ファイル作成

`src-tauri/migrations/` ディレクトリに新しいバージョンの SQL ファイルを作成する．ファイル名は `v{N}.sql` の形式とする．

### 例: v2.sql — tags テーブルの追加

`src-tauri/migrations/v2.sql`:

```sql
-- v2: タグ機能の追加
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#808080',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS character_tags (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (character_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_character_tags_character ON character_tags(character_id);
CREATE INDEX IF NOT EXISTS idx_character_tags_tag ON character_tags(tag_id);
```

### 既存テーブルへのカラム追加

```sql
-- v3: characters テーブルに notes カラムを追加
ALTER TABLE characters ADD COLUMN notes TEXT;
```

## ステップ 2: schema.rs 更新

`src-tauri/src/db/schema.rs` を 2 箇所更新する．

### 2-1. CURRENT_VERSION の更新

```rust
/// Current schema version
const CURRENT_VERSION: i32 = 2;  // 1 → 2 に更新
```

### 2-2. マイグレーション関数の追加

`migrations()` 関数の `Vec` に新しい `Migration` エントリを追加する：

```rust
fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial schema: projects, characters, sprites, workflows, jobs, settings",
            up: migrate_v1,
        },
        // 追加
        Migration {
            version: 2,
            description: "Add tags and character_tags tables",
            up: migrate_v2,
        },
    ]
}
```

### 2-3. マイグレーション関数の実装

同ファイルの末尾にマイグレーション関数を追加する：

```rust
/// Migration v2: Tags
fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v2.sql"))?;
    Ok(())
}
```

### マイグレーションの実行フロー

`schema.rs` の `run_migrations()` 関数は以下の手順で動作する：

1. `schema_version` テーブルから現在のバージョンを取得
2. `migrations()` に登録されたマイグレーションのうち，現在のバージョンより大きいものを順次実行
3. 各マイグレーション実行後，`schema_version` テーブルにバージョンと説明を記録

この関数は `AppState::new()` → `db::initialize()` の流れでアプリ起動時に呼び出される．

### CURRENT_VERSION の整合性チェック

`run_migrations()` には以下のアサーションがある：

```rust
assert!(
    all_migrations.last().map_or(true, |m| m.version == CURRENT_VERSION),
    "CURRENT_VERSION ({}) does not match last migration version",
    CURRENT_VERSION,
);
```

`CURRENT_VERSION` と `migrations()` の最後のエントリの `version` が一致しない場合，パニックする．必ず両方を同時に更新すること．

## ステップ 3: クエリ・モデル・型の同期

スキーマ変更に伴い，以下の 3 箇所を同期する必要がある．

### 3-1. Rust モデル（`src-tauri/src/models/`）

新しいテーブルに対応する構造体を追加する：

`src-tauri/src/models/tag.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    pub name: String,
    pub color: Option<String>,
}
```

`src-tauri/src/models/mod.rs` に登録する：

```rust
pub mod tag;
pub use tag::*;
```

### 3-2. DB クエリ（`src-tauri/src/db/queries/`）

新しいテーブルに対する CRUD クエリを追加する：

`src-tauri/src/db/queries/tag.rs`:

```rust
use rusqlite::{params, Connection};
use crate::models::{Tag, CreateTag};

pub fn create_tag(conn: &Connection, input: &CreateTag) -> Result<Tag, rusqlite::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let color = input.color.as_deref().unwrap_or("#808080");

    conn.execute(
        "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
        params![id, input.name, color],
    )?;

    conn.query_row(
        "SELECT id, name, color, created_at FROM tags WHERE id = ?1",
        params![id],
        |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
}

pub fn list_tags(conn: &Connection) -> Result<Vec<Tag>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, name, color, created_at FROM tags ORDER BY name")?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}
```

### 3-3. TypeScript 型（`src/lib/types/index.ts`）

Rust モデルと同期した TypeScript 型を追加する：

```typescript
// --- Tag ---

export interface Tag {
  id: string;
  name: string;
  color: string;
  created_at: string;
}

export interface CreateTag {
  name: string;
  color?: string;
}
```

## SQLite の制約と注意点

### ALTER TABLE の制限

SQLite の `ALTER TABLE` は以下の操作のみサポートする：

- `ALTER TABLE ... RENAME TO ...` — テーブル名の変更
- `ALTER TABLE ... ADD COLUMN ...` — カラムの追加
- `ALTER TABLE ... RENAME COLUMN ... TO ...` — カラム名の変更（SQLite 3.25.0+）
- `ALTER TABLE ... DROP COLUMN ...` — カラムの削除（SQLite 3.35.0+）

以下の操作はサポートされない：

- カラムの型変更
- NOT NULL 制約の追加・変更
- 外部キー制約の追加・変更
- デフォルト値に非定数式（`datetime('now')` 等）を使ったカラム追加

### カラム型変更が必要な場合

テーブルの再作成で対応する：

```sql
-- 1. 一時テーブルにデータを退避
CREATE TABLE characters_backup AS SELECT * FROM characters;

-- 2. 元テーブルを削除
DROP TABLE characters;

-- 3. 新スキーマでテーブルを再作成
CREATE TABLE characters (
    -- 新しいスキーマ定義
);

-- 4. データを復元
INSERT INTO characters SELECT * FROM characters_backup;

-- 5. 一時テーブルを削除
DROP TABLE characters_backup;
```

> **注意:** 外部キー制約が設定されている場合，`PRAGMA foreign_keys = OFF;` を先に実行する必要がある場合がある．

### JSON データの格納

本プロジェクトでは `directions` と `animations` を TEXT 型カラムに JSON 文字列として格納している：

```sql
-- v1.sql での定義
directions TEXT NOT NULL DEFAULT '["down","left","right","up"]',
animations TEXT NOT NULL DEFAULT '[{"name":"idle","frame_count":2,"frame_duration_ms":300},...]',
```

Rust 側では `serde_json` でシリアライズ/デシリアライズを行う．新しい JSON カラムを追加する場合も同じパターンに従うこと．

### ID の生成

すべてのテーブルの主キーは UUID v4 文字列を使用する：

```rust
let id = uuid::Uuid::new_v4().to_string();
```

### タイムスタンプ

`created_at` / `updated_at` カラムには SQLite の `datetime('now')` をデフォルト値として設定する：

```sql
created_at TEXT NOT NULL DEFAULT (datetime('now')),
updated_at TEXT NOT NULL DEFAULT (datetime('now'))
```

## 開発中の DB リセット方法

開発中にスキーマを大幅に変更した場合やデータを初期化したい場合は，DB ファイルを直接削除してアプリを再起動する．

`{app_data_dir}/figurine_studio.db` を削除すること．`{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリである．

各プラットフォームでの一般的なパス：

| プラットフォーム | パス |
|---------------|------|
| macOS | `~/Library/Application Support/com.figurine-studio.app/` |
| Windows | `%APPDATA%\com.figurine-studio.app\` |
| Linux | `~/.local/share/com.figurine-studio.app/` |

```bash
# macOS の場合
rm ~/Library/Application\ Support/com.figurine-studio.app/figurine_studio.db

# その後アプリを再起動すると，マイグレーションが再実行され新しい DB が作成される
cargo tauri dev
```

> **注意:** DB を削除するとすべてのデータ（プロジェクト，キャラクター，スプライト，設定）が失われる．ファイルシステム上の画像ファイルは削除されないが，DB との紐付けが失われる．

## 完全なコード例: マイグレーション追加

以下に，`characters` テーブルに `notes` カラムを追加する完全な例を示す．

### 1. SQL ファイル（`src-tauri/migrations/v2.sql`）

```sql
-- v2: characters テーブルに notes カラムを追加
ALTER TABLE characters ADD COLUMN notes TEXT;
```

### 2. schema.rs 更新（`src-tauri/src/db/schema.rs`）

```rust
/// Current schema version
const CURRENT_VERSION: i32 = 2;

fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial schema: projects, characters, sprites, workflows, jobs, settings",
            up: migrate_v1,
        },
        Migration {
            version: 2,
            description: "Add notes column to characters",
            up: migrate_v2,
        },
    ]
}

// ... 既存の migrate_v1 ...

/// Migration v2: Add notes to characters
fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v2.sql"))?;
    Ok(())
}
```

### 3. Rust モデル更新（`src-tauri/src/models/character.rs`）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub category: String,
    pub custom_prompt: Option<String>,
    pub notes: Option<String>,          // 追加
    pub status: String,
    pub spritesheet_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCharacter {
    pub name: Option<String>,
    pub category: Option<String>,
    pub custom_prompt: Option<String>,
    pub notes: Option<String>,          // 追加
}
```

### 4. TypeScript 型更新（`src/lib/types/index.ts`）

```typescript
export interface Character {
  id: string;
  project_id: string;
  name: string;
  category: CharacterCategory;
  custom_prompt: string | null;
  notes: string | null;          // 追加
  status: CharacterStatus;
  spritesheet_path: string | null;
  created_at: string;
  updated_at: string;
}

export interface UpdateCharacter {
  name?: string;
  category?: CharacterCategory;
  custom_prompt?: string;
  notes?: string;                // 追加
}
```

### 5. DB クエリ更新

関連する SELECT 文・INSERT 文・UPDATE 文に `notes` カラムを追加する．既存のクエリファイル（`src-tauri/src/db/queries/character.rs`）を確認し，カラムの追加漏れがないよう注意すること．
