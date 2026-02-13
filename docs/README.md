# Figurine Studio ドキュメント

Figurine Studio の開発者向けドキュメントハブ．

> プロジェクト概要は [README.md](../README.md) を，Claude Code 向けガイダンスは [.claude/CLAUDE.md](../.claude/CLAUDE.md) を参照．

---

## アーキテクチャ

| ドキュメント | 概要 |
|---|---|
| [アーキテクチャ全体図](architecture/overview.md) | 2プロセスモデル（Frontend ↔ Backend），フロントエンド/バックエンド構成，セキュリティ |
| [データフロー](architecture/data-flow.md) | スプライトパイプライン（Import → AI Texture → BG Removal → Spritesheet → Bevy Export） |
| [DB 設計](architecture/database.md) | テーブル定義，ER 図，インデックス，マイグレーションの仕組み |

## 起動シーケンス

| ドキュメント | 概要 |
|---|---|
| [システム起動シーケンス](startup/startup-sequence.md) | 開発モード・本番ビルドの起動フロー，Rust/フロントエンド初期化，SetupWizard |

## 外部システム連携

| ドキュメント | 概要 |
|---|---|
| [ComfyUI 連携ガイド](external/comfyui.md) | API エンドポイント，ワークフロープレースホルダ，バッチ処理，リトライ戦略 |
| [ONNX Runtime / U2-Net ガイド](external/onnx-runtime.md) | モデル仕様，推論パイプライン，セッション設定，バッチ処理 |

## 開発ガイド

| ドキュメント | 概要 |
|---|---|
| [開発環境セットアップ](guides/setup.md) | 前提条件，セットアップ手順，ビルド，トラブルシューティング |
| [Tauri コマンド追加手順](guides/adding-tauri-command.md) | Rust コマンド定義 → lib.rs 登録 → tauri.ts ラッパー → 型定義の4ステップ |
| [フロントエンド機能追加手順](guides/adding-frontend-feature.md) | ルート追加，Store パターン，コンポーネント，テーマ対応，日本語 UI 規約 |
| [DB マイグレーション追加手順](guides/adding-db-migration.md) | SQL ファイル作成 → schema.rs 更新 → クエリ/モデル/型の同期 |
