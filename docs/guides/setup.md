# 開発環境セットアップ

Figurine Studio の開発環境を構築する手順を説明する．

## 必須前提条件

以下のツールが事前にインストールされている必要がある．

| ツール | バージョン | 用途 |
|--------|-----------|------|
| **Rust** (stable) | latest stable | バックエンド（Tauri v2）のビルド |
| **Node.js** | v18 以上 | フロントエンド（SvelteKit）のビルド |
| **pnpm** | v8 以上 | フロントエンドのパッケージ管理 |

Rust のインストールは [rustup](https://rustup.rs/) を推奨する．

```bash
# Rust のインストール（未導入の場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Tauri v2 に必要なシステム依存（macOS の場合 Xcode Command Line Tools）
xcode-select --install
```

> **注意:** Tauri v2 はプラットフォーム固有のシステム依存がある．
> 詳細は [Tauri 公式ドキュメント](https://v2.tauri.app/start/prerequisites/) を参照すること．

## 任意前提条件

| ツール | 用途 |
|--------|------|
| **ComfyUI** | AI テクスチャ生成機能を使用する場合に必要．デフォルトのエンドポイントは `http://127.0.0.1:8188`．CSP 設定（`src-tauri/tauri.conf.json`）で許可済み |
| **ネットワーク接続** | ONNX モデル（U2-Net）のダウンロードに必要．初回起動時の SetupWizard でダウンロードされる |

## セットアップ手順

### 1. リポジトリのクローン

```bash
git clone <repository-url>
cd rs-figure-studio
```

### 2. フロントエンド依存のインストール

```bash
pnpm install
```

`package.json` に定義された依存がインストールされる．主要な依存は以下の通り：

- `@tauri-apps/api` — Tauri フロントエンド API
- `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-shell` — Tauri プラグイン
- `svelte` (v5), `@sveltejs/kit` (v2) — フロントエンドフレームワーク
- `lucide-svelte` — アイコンライブラリ

### 3. Rust バックエンドの確認

```bash
cd src-tauri && cargo check && cd ..
```

`Cargo.toml` に定義されたクレートがダウンロード・コンパイルされる．主要な依存は以下の通り：

- `tauri` (v2) — デスクトップアプリフレームワーク
- `rusqlite` (bundled) — SQLite データベース
- `ort` (v2.0.0-rc.11) — ONNX Runtime（U2-Net 背景除去用）
- `image` / `imageproc` — 画像処理
- `reqwest` — HTTP クライアント（ComfyUI 連携用）

### 4. 開発サーバーの起動

```bash
cargo tauri dev
```

このコマンドは以下を同時に実行する：

1. Vite 開発サーバーを `http://localhost:5173` で起動（`beforeDevCommand: "pnpm dev"`）
2. Rust バックエンドをビルドして Tauri ウィンドウを起動

ウィンドウサイズは `1280x800`（最小 `1024x768`）で起動する（`src-tauri/tauri.conf.json` で設定）．

## 初回起動時の SetupWizard フロー

アプリを初めて起動すると，**SetupWizard** が自動的に表示される．以下の手順でセットアップを行う：

### ステップ 1: ONNX モデルのダウンロード

- U2-Net モデル（背景除去用）のダウンロードが求められる
- ダウンロードされたモデルは `{app_data_dir}/models/u2net.onnx` に保存される
- `{app_data_dir}` は Tauri の `app_data_dir()` API で取得されるプラットフォーム固有のディレクトリ
- ダウンロード進捗は `download-progress` イベントでフロントエンドに通知される
- ネットワーク接続が必要

### ステップ 2: ComfyUI 接続設定

- ComfyUI のエンドポイント URL を設定する（デフォルト: `http://127.0.0.1:8188`）
- 接続テストを実行して ComfyUI サーバーの到達性を確認する
- ComfyUI を使用しない場合はスキップ可能

セットアップの完了状態は `{app_data_dir}/figurine_studio.db` 内の `app_settings` テーブルに保存される．

## ビルドコマンド

### プロダクションビルド

```bash
cargo tauri build
```

フロントエンドのビルド（`pnpm build`）と Rust バックエンドのリリースビルドが順次実行され，プラットフォーム固有のインストーラが生成される．

## チェックコマンド

### フロントエンド型チェック

```bash
pnpm check
```

`svelte-kit sync` でルーティング型を同期した後，`svelte-check` で TypeScript 型チェックを実行する．

### Rust リント

```bash
cd src-tauri && cargo clippy
```

### Rust フォーマットチェック

```bash
cd src-tauri && cargo fmt --check
```

## トラブルシューティング

### `cargo tauri dev` で Vite の起動に失敗する

**原因:** フロントエンド依存が未インストール

```bash
pnpm install
```

### ONNX モデルのダウンロードが失敗する

**原因:** ネットワーク接続の問題，またはプロキシ設定

- ネットワーク接続を確認する
- 企業プロキシ環境の場合，`HTTP_PROXY` / `HTTPS_PROXY` 環境変数を設定する
- 手動ダウンロード後，`{app_data_dir}/models/u2net.onnx` に配置することでも対応可能

### ComfyUI に接続できない

**原因:** ComfyUI サーバーが起動していない，またはポートが異なる

- ComfyUI が `http://127.0.0.1:8188` で起動していることを確認する
- 別のポートを使用している場合，アプリの設定画面からエンドポイントを変更する
- CSP 設定（`src-tauri/tauri.conf.json`）は `127.0.0.1:8188` のみ許可している点に注意

### SQLite DB の破損

**対処:** DB ファイルを削除してアプリを再起動する．

`{app_data_dir}/figurine_studio.db` を削除すると，次回起動時にマイグレーションが再実行され，新しい DB が作成される．ただし，全てのデータが失われるため注意すること．

### `ort` クレートのビルドに失敗する

**原因:** ONNX Runtime のバイナリダウンロードに問題がある

- `ort` クレートは `download-binaries` feature を使用しており，ビルド時に ONNX Runtime のプリビルドバイナリをダウンロードする
- ネットワーク接続を確認する
- キャッシュをクリアして再試行する:

```bash
cd src-tauri && cargo clean && cargo build
```

### ウィンドウが表示されない（macOS）

**原因:** Xcode Command Line Tools が未インストール

```bash
xcode-select --install
```
