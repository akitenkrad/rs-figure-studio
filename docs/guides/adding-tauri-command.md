# 新規 Tauri コマンド追加手順

Figurine Studio に新しい Tauri コマンド（バックエンド API）を追加する手順を説明する．

## 概要

Tauri コマンドは，フロントエンド（SvelteKit）からバックエンド（Rust）を呼び出すための RPC インターフェースである．新規コマンドの追加は以下の 4 ステップで行う：

1. **Rust コマンド定義** — `src-tauri/src/commands/` にコマンド関数を実装
2. **lib.rs 登録** — `src-tauri/src/lib.rs` の `generate_handler![]` にコマンドを登録
3. **tauri.ts ラッパー** — `src/lib/api/tauri.ts` に型付き `invoke()` ラッパーを追加
4. **types/index.ts 型定義** — `src/lib/types/index.ts` に Rust モデルと同期した型を追加

## ステップ 1: Rust コマンド定義

### コマンドファイルの構成

コマンドは `src-tauri/src/commands/` ディレクトリ内にドメインごとのファイルで管理される．新規ドメインの場合はファイルを追加し，`src-tauri/src/commands/mod.rs` に登録する．

既存のコマンドファイル:

| ファイル | ドメイン |
|---------|---------|
| `project_commands.rs` | プロジェクト CRUD |
| `character_commands.rs` | キャラクター CRUD + スプライトインポート |
| `sprite_commands.rs` | スプライト一覧・割り当て更新 |
| `image_processing.rs` | 画像正規化 |
| `background_removal.rs` | ONNX 背景除去 |
| `model_download.rs` | ONNX モデルダウンロード |
| `spritesheet.rs` | スプライトシート生成 |
| `bevy_export.rs` | Bevy Engine エクスポート |
| `comfyui.rs` | ComfyUI ワークフロー管理・バッチ処理 |
| `settings_commands.rs` | アプリ設定の取得・更新 |

### 基本パターン

すべてのコマンドは以下のパターンに従う：

```rust
use crate::db;
use crate::error::AppError;
use crate::models::YourModel;
use crate::state::AppState;

#[tauri::command]
pub async fn your_command_name(
    state: tauri::State<'_, AppState>,
    input_param: String,
) -> Result<YourModel, AppError> {
    // 1. DB ロックを取得
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2. バリデーション
    if input_param.trim().is_empty() {
        return Err(AppError::Validation("パラメータが必要です".into()));
    }

    // 3. DB クエリを呼び出し
    let result = db::queries::your_module::your_query(&conn, &input_param)?;

    // 4. 結果を返却
    Ok(result)
}
```

**ポイント:**

- `#[tauri::command]` 属性マクロを付与する
- 関数は `pub async fn` で定義する
- `tauri::State<'_, AppState>` で共有ステートを受け取る
- 戻り値は `Result<T, AppError>` とする
- `AppState.db` は `Mutex<Connection>` なので，使用時に `lock()` する

### 進捗イベント付きコマンドのパターン

長時間実行されるコマンドでは，`tauri::AppHandle` を受け取り，進捗イベントを emit する：

```rust
use tauri::Emitter;

#[tauri::command]
pub async fn your_batch_command(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
) -> Result<Vec<String>, AppError> {
    let items = get_items_to_process()?;
    let total = items.len() as u32;
    let mut results = Vec::new();

    for (i, item) in items.iter().enumerate() {
        // 処理を実行
        let result = process_item(item)?;
        results.push(result);

        // 進捗イベントを送信
        let _ = app.emit(
            "your-progress-event",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "item_id": item.id,
            }),
        );
    }

    Ok(results)
}
```

**既存の進捗イベント名:**

| イベント名 | 用途 | ペイロード |
|-----------|------|-----------|
| `download-progress` | ONNX モデルダウンロード | `{ downloaded, total, percentage }` |
| `processing-progress` | 画像正規化処理 | `{ stage, current, total }` |
| `comfyui-progress` | ComfyUI バッチ処理 | `{ current, total, status, current_file }` |
| `bg-removal-progress` | 背景除去バッチ処理 | `{ current, total, sprite_id }` |

### エラーハンドリング

`AppError` enum（`src-tauri/src/error.rs`）を使用する．各バリアントは日本語のエラーメッセージを含む：

```rust
// 利用可能なエラーバリアント
AppError::NotFound(String)        // リソースが見つからない
AppError::Validation(String)      // 入力値バリデーションエラー
AppError::Database(String)        // データベース操作エラー
AppError::Io(String)              // ファイル I/O エラー
AppError::Onnx(String)            // ONNX ランタイムエラー
AppError::ComfyUI(String)         // ComfyUI 連携エラー
AppError::ImageProcessing(String) // 画像処理エラー
AppError::Export(String)          // エクスポート処理エラー
AppError::Internal(String)        // 予期しない内部エラー
```

`rusqlite::Error`，`std::io::Error`，`serde_json::Error`，`anyhow::Error` からの `From` 変換が実装されているため，`?` 演算子で自動変換される．

## ステップ 2: lib.rs 登録

`src-tauri/src/lib.rs` の `generate_handler![]` マクロにコマンドのフルパスを追加する：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 既存コマンド ...
    // YourDomain
    commands::your_module::your_command_name,
])
```

新規ファイルの場合は `src-tauri/src/commands/mod.rs` にもモジュールを登録する：

```rust
pub mod your_module;
```

## ステップ 3: tauri.ts ラッパー

`src/lib/api/tauri.ts` にフロントエンドから呼び出すための型付きラッパー関数を追加する：

```typescript
// --- YourDomain API ---

export const yourDomainApi = {
  async yourCommandName(inputParam: string): Promise<YourModel> {
    return invoke<YourModel>('your_command_name', { inputParam });
  },
};
```

**命名規約:**

| 層 | 規約 | 例 |
|----|------|-----|
| Rust コマンド名 | `snake_case` | `create_project` |
| TypeScript ラッパー名 | `camelCase` | `createProject` |
| `invoke()` のコマンド名文字列 | Rust 側と同じ `snake_case` | `'create_project'` |
| `invoke()` のパラメータキー | Rust 側は `snake_case`，TS 側は `camelCase` | Rust: `character_id` / TS: `{ characterId }` |

> **注意:** Tauri v2 では `invoke()` のパラメータ名が自動的に camelCase から snake_case に変換される．
> TypeScript 側では camelCase でパラメータを渡し，Rust 側では snake_case で受け取る．

### 進捗イベントリスナーの追加

進捗イベント付きコマンドの場合，イベントリスナーヘルパーも追加する：

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface YourProgress {
  current: number;
  total: number;
  item_id: string;
}

export function onYourProgress(
  callback: (progress: YourProgress) => void,
): Promise<UnlistenFn> {
  return listen<YourProgress>('your-progress-event', (event) => {
    callback(event.payload);
  });
}
```

## ステップ 4: types/index.ts 型定義

`src/lib/types/index.ts` に Rust のモデル構造体と同期した TypeScript 型を追加する：

```typescript
// Rust の struct YourModel に対応
export interface YourModel {
  id: string;
  name: string;
  created_at: string;  // Rust の String → TS の string
  count: number;       // Rust の i32/u32 → TS の number
  weight: number;      // Rust の f64 → TS の number
  optional_field: string | null;  // Rust の Option<String> → TS の string | null
}

// Rust の CreateYourModel に対応
export interface CreateYourModel {
  name: string;
  count?: number;  // Rust の Option<i32> → TS の number?（省略可能）
}
```

**Rust → TypeScript 型マッピング:**

| Rust 型 | TypeScript 型 |
|---------|--------------|
| `String` | `string` |
| `i32`, `u32`, `i64`, `f64` | `number` |
| `bool` | `boolean` |
| `Option<T>` | `T \| null`（レスポンス時）/ `T?`（リクエスト時） |
| `Vec<T>` | `T[]` |
| `serde_json::Value` | `string`（JSON 文字列として） |

## 既存コマンド一覧

`src-tauri/src/lib.rs` の `generate_handler![]` に登録されている全コマンド：

### Project

| コマンド | 概要 |
|---------|------|
| `create_project` | プロジェクト作成 |
| `get_project` | プロジェクト取得 |
| `list_projects` | プロジェクト一覧 |
| `update_project` | プロジェクト更新 |
| `delete_project` | プロジェクト削除（CASCADE + ファイル削除） |

### Character

| コマンド | 概要 |
|---------|------|
| `create_character` | キャラクター作成 |
| `get_character` | キャラクター取得 |
| `list_characters` | キャラクター一覧（project_id 指定） |
| `update_character` | キャラクター更新 |
| `delete_character` | キャラクター削除 |
| `import_sprites` | スプライト画像のインポート |

### Image Processing

| コマンド | 概要 |
|---------|------|
| `normalize_sprite` | 単一スプライトの正規化 |
| `normalize_batch` | バッチ正規化 |

### Model Download

| コマンド | 概要 |
|---------|------|
| `check_onnx_model` | ONNX モデルの状態確認 |
| `download_onnx_model` | ONNX モデルのダウンロード |

### Background Removal

| コマンド | 概要 |
|---------|------|
| `initialize_onnx` | ONNX Runtime の初期化 |
| `remove_background` | 単一スプライトの背景除去 |
| `remove_background_batch` | バッチ背景除去（進捗イベント付き） |

### Spritesheet

| コマンド | 概要 |
|---------|------|
| `generate_spritesheet` | スプライトシート生成 |

### Bevy Export

| コマンド | 概要 |
|---------|------|
| `export_for_bevy` | Bevy Engine 用エクスポート |

### ComfyUI

| コマンド | 概要 |
|---------|------|
| `check_comfyui_connection` | ComfyUI 接続確認 |
| `list_workflows` | ワークフロー一覧 |
| `create_workflow` | ワークフロー作成 |
| `import_workflow` | ワークフローインポート |
| `update_workflow` | ワークフロー更新 |
| `delete_workflow` | ワークフロー削除 |
| `set_default_workflow` | デフォルトワークフロー設定 |
| `process_batch_comfyui` | ComfyUI バッチ処理（進捗イベント付き） |

### Settings

| コマンド | 概要 |
|---------|------|
| `get_settings` | 全設定の取得 |
| `update_setting` | 設定値の更新 |

### Sprite

| コマンド | 概要 |
|---------|------|
| `list_sprites` | スプライト一覧（character_id 指定） |
| `update_sprite_assignment` | スプライトの方向・アニメーション割り当て更新 |

## 完全なコード例: 新規コマンド追加

以下に，「プロジェクトのスプライト数を取得する」コマンドを追加する完全な例を示す．

### 1. Rust コマンド（`src-tauri/src/commands/project_commands.rs` に追加）

```rust
#[tauri::command]
pub async fn get_project_sprite_count(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<u32, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // プロジェクトの存在確認
    db::queries::project::get_project(&conn, &project_id)?
        .ok_or_else(|| AppError::NotFound(format!("Project not found: {}", project_id)))?;

    // スプライト数をカウント
    let count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sprites s
             JOIN characters c ON s.character_id = c.id
             WHERE c.project_id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(count)
}
```

### 2. lib.rs 登録（`src-tauri/src/lib.rs`）

```rust
.invoke_handler(tauri::generate_handler![
    // Project
    commands::project_commands::create_project,
    // ... 既存コマンド ...
    commands::project_commands::get_project_sprite_count,  // 追加
])
```

### 3. tauri.ts ラッパー（`src/lib/api/tauri.ts`）

```typescript
export const projectApi = {
  // ... 既存メソッド ...

  async getSpriteCount(projectId: string): Promise<number> {
    return invoke<number>('get_project_sprite_count', { projectId });
  },
};
```

### 4. 型定義（`src/lib/types/index.ts`）

この例では `number` のみを返すため，追加の型定義は不要である．カスタム型が必要な場合は `interface` を追加する．
