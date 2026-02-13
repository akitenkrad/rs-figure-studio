# フロントエンド機能追加手順

Figurine Studio のフロントエンド（SvelteKit + Svelte 5）に新しい機能を追加する手順を説明する．

## アーキテクチャ概要

```
src/
├── routes/                  # SvelteKit ファイルベースルーティング
│   ├── +layout.svelte       # ルートレイアウト（Sidebar + Toast + SetupWizard）
│   ├── +page.svelte         # ホーム画面（プロジェクト一覧）
│   ├── project/
│   │   ├── new/+page.svelte
│   │   └── [id]/
│   │       ├── +layout.svelte
│   │       └── +page.svelte
│   ├── character/
│   │   ├── new/+page.svelte
│   │   └── [id]/
│   │       ├── +layout.svelte  # タブレイアウト
│   │       ├── +page.svelte    # キャラクター詳細
│   │       ├── import/+page.svelte
│   │       ├── process/+page.svelte
│   │       └── preview/+page.svelte
│   └── settings/
│       ├── +page.svelte
│       ├── models/+page.svelte
│       └── comfyui/+page.svelte
├── lib/
│   ├── api/tauri.ts          # Tauri invoke ラッパー
│   ├── types/index.ts        # TypeScript 型定義
│   ├── stores/               # Svelte 5 runes ベースの状態管理
│   │   ├── project.svelte.ts
│   │   ├── character.svelte.ts
│   │   ├── sprite.svelte.ts
│   │   ├── onnx.svelte.ts
│   │   ├── comfyui.svelte.ts
│   │   ├── settings.svelte.ts
│   │   ├── setup.svelte.ts
│   │   └── toast.svelte.ts
│   └── components/           # 再利用可能コンポーネント
│       ├── Sidebar.svelte
│       ├── Breadcrumb.svelte
│       ├── Toast.svelte
│       ├── ProjectCard.svelte
│       ├── ImageDropZone.svelte
│       ├── ModelDownloader.svelte
│       ├── SpritePreview.svelte
│       ├── WorkflowManager.svelte
│       ├── ProgressBar.svelte
│       ├── SetupWizard.svelte
│       ├── ComfyUIGuide.svelte
│       ├── LoadingSpinner.svelte
│       ├── EmptyState.svelte
│       ├── ErrorBoundary.svelte
│       └── ConfirmDialog.svelte
└── app.css                   # グローバルスタイル（テーマ変数含む）
```

## ルート追加

SvelteKit のファイルベースルーティングに従ってルートを追加する．

### 静的ルートの追加

`src/routes/your-feature/+page.svelte` を作成する：

```svelte
<script lang="ts">
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import type { BreadcrumbItem } from '$lib/types';

  const breadcrumbs: BreadcrumbItem[] = [
    { label: 'ホーム', href: '/' },
    { label: '新機能' },
  ];
</script>

<Breadcrumb items={breadcrumbs} />

<div class="page-header">
  <h1>新機能ページ</h1>
</div>

<div class="page-content">
  <!-- ページコンテンツ -->
</div>

<style>
  .page-header {
    margin-bottom: var(--space-6);
  }
  .page-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
</style>
```

### 動的ルートの追加

パラメータ付きルートは `[param]` ディレクトリ名で作成する．

`src/routes/your-feature/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/stores';

  // URL パラメータの取得
  const id = $page.params.id;
</script>

<h1>アイテム: {id}</h1>
```

### レイアウトの追加

サブルート間で共有するレイアウトは `+layout.svelte` で定義する．キャラクター詳細ページのタブナビゲーション（`src/routes/character/[id]/+layout.svelte`）がこのパターンの実例である．

```svelte
<script lang="ts">
  let { children } = $props();
</script>

<nav class="tab-nav">
  <!-- タブナビゲーション -->
</nav>

<div class="tab-content">
  {@render children()}
</div>
```

### SPA フォールバック

本プロジェクトは `adapter-static` を使用した SPA モードで動作する．`src/routes/+layout.ts` で SSR が無効化されている：

```typescript
export const ssr = false;
export const prerender = false;
```

## Store パターン

### Svelte 5 Runes ベースの状態管理

本プロジェクトでは外部の状態管理ライブラリを使用せず，Svelte 5 の runes（`$state`，`$derived`）を使ったシングルトンパターンで状態管理を行う．

### Store の基本構造

`src/lib/stores/your-feature.svelte.ts`:

```typescript
import type { YourModel, CreateYourModel } from '$lib/types';
import { yourApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
// モジュールレベルの $state 変数（プライベート）
let items = $state<YourModel[]>([]);
let currentItem = $state<YourModel | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

// --- Derived ---
// $state から派生する計算値
const itemCount = $derived(items.length);
const hasItems = $derived(items.length > 0);

// --- Actions ---
// 非同期アクション関数
async function loadItems(): Promise<void> {
  loading = true;
  error = null;
  try {
    items = await yourApi.list();
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `一覧の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function createItem(data: CreateYourModel): Promise<YourModel | null> {
  loading = true;
  error = null;
  try {
    const item = await yourApi.create(data);
    items = [...items, item];
    toastStore.addToast('success', 'アイテムを作成しました');
    return item;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `アイテムの作成に失敗しました: ${e}`);
    return null;
  } finally {
    loading = false;
  }
}

// --- Export ---
// シングルトンオブジェクトとして export
// getter で $state/$derived のリアクティビティを維持する
export const yourFeatureStore = {
  get items() { return items; },
  get currentItem() { return currentItem; },
  get loading() { return loading; },
  get error() { return error; },
  get itemCount() { return itemCount; },
  get hasItems() { return hasItems; },
  loadItems,
  createItem,
};
```

### Store パターンのポイント

1. **`$state` はモジュールレベルで宣言する** — コンポーネント内ではなく，ファイルのトップレベルに配置する
2. **`$derived` で派生値を定義する** — `$state` から計算される値は `$derived` を使う
3. **シングルトンオブジェクトで export する** — `get` アクセサを使い，リアクティビティを維持する
4. **Toast で結果を通知する** — `toastStore.addToast()` でユーザーに処理結果を表示する
5. **エラーを握りつぶさない** — `try/catch` で `error` state に格納し，UI で参照可能にする

### 既存 Store 一覧

| Store | ファイル | 用途 |
|-------|---------|------|
| `projectStore` | `project.svelte.ts` | プロジェクト CRUD・一覧管理 |
| `characterStore` | `character.svelte.ts` | キャラクター CRUD・一覧管理 |
| `spriteStore` | `sprite.svelte.ts` | スプライト一覧・割り当て管理 |
| `onnxStore` | `onnx.svelte.ts` | ONNX モデル状態・背景除去操作 |
| `comfyuiStore` | `comfyui.svelte.ts` | ComfyUI 接続・ワークフロー管理 |
| `settingsStore` | `settings.svelte.ts` | アプリ設定（テーマ等） |
| `setupStore` | `setup.svelte.ts` | 初回セットアップウィザード状態 |
| `toastStore` | `toast.svelte.ts` | トースト通知管理 |

## コンポーネント追加

### 基本パターン

`src/lib/components/YourComponent.svelte`:

```svelte
<script lang="ts">
  // $props() で型安全な props を受け取る
  interface Props {
    title: string;
    count?: number;
    onAction?: (id: string) => void;
  }

  let { title, count = 0, onAction }: Props = $props();

  // ローカルステート
  let isExpanded = $state(false);

  // $effect で副作用を処理
  $effect(() => {
    console.log(`Title changed: ${title}`);
  });

  function handleClick() {
    isExpanded = !isExpanded;
    onAction?.('some-id');
  }
</script>

<div class="your-component">
  <h3>{title}</h3>
  <span class="badge badge-sm">{count}</span>
  <button class="btn btn-secondary" onclick={handleClick}>
    {isExpanded ? '閉じる' : '開く'}
  </button>
</div>

<style>
  .your-component {
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: var(--space-4);
  }
</style>
```

### Svelte 5 のポイント

- **`$props()`**: Svelte 5 では `export let` の代わりに `$props()` を使用する
- **`$state()`**: コンポーネント内のリアクティブな変数宣言
- **`$effect()`**: 副作用処理（Svelte 4 の `$:` reactive statement に相当）
- **`$derived()`**: 派生値の計算
- **`{@render children()}`**: スロットの代わりにスニペットレンダリングを使用する

### 既存コンポーネント一覧

| コンポーネント | 用途 |
|--------------|------|
| `Sidebar.svelte` | サイドバーナビゲーション（折りたたみ対応） |
| `Breadcrumb.svelte` | パンくずナビゲーション |
| `Toast.svelte` | トースト通知表示 |
| `ProjectCard.svelte` | プロジェクトカード表示 |
| `ImageDropZone.svelte` | 画像ドラッグ&ドロップ領域 |
| `ModelDownloader.svelte` | ONNX モデルダウンロード UI |
| `SpritePreview.svelte` | スプライト画像プレビュー |
| `WorkflowManager.svelte` | ComfyUI ワークフロー管理 UI |
| `ProgressBar.svelte` | プログレスバー |
| `SetupWizard.svelte` | 初回セットアップウィザード |
| `ComfyUIGuide.svelte` | ComfyUI セットアップガイド |
| `LoadingSpinner.svelte` | ローディングスピナー |
| `EmptyState.svelte` | 空状態の表示 |
| `ErrorBoundary.svelte` | エラーバウンダリ |
| `ConfirmDialog.svelte` | 確認ダイアログ |

## 日本語 UI 規約

**すべてのユーザー向けメッセージは日本語で記述する．**

### Toast メッセージ

```typescript
// 成功メッセージ
toastStore.addToast('success', 'プロジェクトを作成しました');

// エラーメッセージ（エラー詳細を付加）
toastStore.addToast('error', `プロジェクトの取得に失敗しました: ${e}`);

// 警告メッセージ
toastStore.addToast('warning', '未保存の変更があります');

// 情報メッセージ
toastStore.addToast('info', 'モデルのダウンロードを開始します');
```

### ラベル・ボタンテキスト

```svelte
<label class="label">プロジェクト名</label>
<input class="input" placeholder="例: RPGキャラクター" />

<button class="btn btn-primary">作成</button>
<button class="btn btn-secondary">キャンセル</button>
<button class="btn btn-danger">削除</button>
```

### バリデーションメッセージ

```svelte
{#if nameError}
  <p class="error-text">プロジェクト名は必須です</p>
{/if}
```

## テーマ対応

### CSS カスタムプロパティ

すべてのスタイルは `src/app.css` で定義された CSS カスタムプロパティ（CSS 変数）を使用する．ハードコードされた色値を直接使用してはならない．

### 主要なテーマ変数

```css
/* 背景色 */
var(--bg-primary)     /* メイン背景 */
var(--bg-secondary)   /* カード・セクション背景 */
var(--bg-tertiary)    /* ホバー・サブ要素背景 */
var(--bg-elevated)    /* モーダル・トースト背景 */

/* テキスト色 */
var(--text-primary)   /* 主要テキスト */
var(--text-secondary) /* 補助テキスト */
var(--text-muted)     /* 薄いテキスト・プレースホルダー */

/* アクセントカラー */
var(--accent-primary)       /* メインアクセント（オレンジ系） */
var(--accent-primary-hover) /* ホバー時 */
var(--accent-success)       /* 成功（緑） */
var(--accent-warning)       /* 警告（黄） */
var(--accent-danger)        /* 危険（赤） */
var(--accent-info)          /* 情報（青） */

/* ボーダー */
var(--border-default)  /* 標準ボーダー */
var(--border-focus)    /* フォーカスボーダー */

/* シャドウ */
var(--shadow-sm)  /* 小さいシャドウ */
var(--shadow-md)  /* 中シャドウ */
var(--shadow-lg)  /* 大きいシャドウ */

/* スペーシング */
var(--space-1) ~ var(--space-12)  /* 0.25rem 〜 3rem */

/* タイポグラフィ */
var(--text-xs) ~ var(--text-3xl)  /* フォントサイズ */
var(--font-sans)  /* UI フォント */
var(--font-mono)  /* コードフォント */
```

### テーマ切り替え

`data-theme` 属性を `<html>` 要素に設定することでテーマが切り替わる：

- `data-theme="dark"` — ダークテーマ（デフォルト）
- `data-theme="light"` — ライトテーマ

テーマの状態は `settingsStore` が管理し，`app_settings` テーブルに永続化される．

### グローバル CSS クラス

`src/app.css` には再利用可能な CSS クラスが定義されている：

```html
<!-- ボタン -->
<button class="btn btn-primary">メイン操作</button>
<button class="btn btn-secondary">サブ操作</button>
<button class="btn btn-ghost">ゴースト</button>
<button class="btn btn-danger">削除</button>

<!-- カード -->
<div class="card">
  <div class="card-header">...</div>
  <div class="card-body">...</div>
  <div class="card-footer">...</div>
</div>

<!-- フォーム -->
<label class="label">ラベル</label>
<input class="input" />
<textarea class="textarea"></textarea>
<select class="select">...</select>
<div class="form-group">...</div>

<!-- バッジ -->
<span class="badge badge-draft">下書き</span>
<span class="badge badge-importing">インポート中</span>
<span class="badge badge-processing">処理中</span>
<span class="badge badge-complete">完了</span>

<!-- グリッド -->
<div class="card-grid">...</div>   <!-- カードグリッド -->
<div class="image-grid">...</div>  <!-- 画像グリッド -->

<!-- スプライトプレビュー背景 -->
<div class="checkerboard-bg">...</div>
```

## アイコン

本プロジェクトでは [lucide-svelte](https://lucide.dev/) をアイコンライブラリとして使用する．

```svelte
<script lang="ts">
  import { Plus, Trash2, Settings, FolderOpen } from 'lucide-svelte';
</script>

<button class="btn btn-primary">
  <Plus size={16} />
  新規作成
</button>

<button class="btn btn-danger">
  <Trash2 size={16} />
  削除
</button>
```

アイコンは `lucide-svelte` パッケージ（v0.469）から名前付きインポートで使用する．アイコンの一覧は [Lucide Icons](https://lucide.dev/icons/) を参照すること．

## 完全なコード例: 新機能の追加

以下に，「統計ダッシュボード」ページを追加する完全な例を示す．

### 1. Store の作成（`src/lib/stores/stats.svelte.ts`）

```typescript
import { projectApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

interface Stats {
  projectCount: number;
  characterCount: number;
}

let stats = $state<Stats>({ projectCount: 0, characterCount: 0 });
let loading = $state(false);

async function loadStats(): Promise<void> {
  loading = true;
  try {
    const projects = await projectApi.list();
    stats = {
      projectCount: projects.length,
      characterCount: projects.reduce((sum, p) => sum + (p.character_count ?? 0), 0),
    };
  } catch (e) {
    toastStore.addToast('error', `統計の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

export const statsStore = {
  get stats() { return stats; },
  get loading() { return loading; },
  loadStats,
};
```

### 2. ルートの作成（`src/routes/stats/+page.svelte`）

```svelte
<script lang="ts">
  import { BarChart3 } from 'lucide-svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import { statsStore } from '$lib/stores/stats.svelte';
  import type { BreadcrumbItem } from '$lib/types';

  const breadcrumbs: BreadcrumbItem[] = [
    { label: 'ホーム', href: '/' },
    { label: '統計' },
  ];

  $effect(() => {
    statsStore.loadStats();
  });
</script>

<Breadcrumb items={breadcrumbs} />

<div class="page-header">
  <BarChart3 size={24} />
  <h1>統計ダッシュボード</h1>
</div>

{#if statsStore.loading}
  <LoadingSpinner />
{:else}
  <div class="card-grid">
    <div class="card">
      <div class="card-header">
        <h3>プロジェクト数</h3>
      </div>
      <div class="card-body">
        <p class="stat-value">{statsStore.stats.projectCount}</p>
      </div>
    </div>
    <div class="card">
      <div class="card-header">
        <h3>キャラクター数</h3>
      </div>
      <div class="card-body">
        <p class="stat-value">{statsStore.stats.characterCount}</p>
      </div>
    </div>
  </div>
{/if}

<style>
  .page-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-6);
  }
  .stat-value {
    font-size: var(--text-3xl);
    font-weight: var(--font-weight-bold);
    color: var(--accent-primary);
  }
</style>
```
