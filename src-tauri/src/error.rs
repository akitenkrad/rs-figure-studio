use serde::Serialize;
use thiserror::Error;

/// アプリケーション全体のエラー型
///
/// 各バリアントはユーザー向け日本語メッセージを含み，
/// フロントエンドでの表示にも利用される．
/// `Serialize` を実装しており，Tauri コマンドの戻り値として
/// そのまま返却可能．
#[derive(Debug, Error, Serialize)]
pub enum AppError {
    /// リソースが見つからない（404相当）
    #[error("見つかりません: {0}")]
    NotFound(String),

    /// 入力値のバリデーションエラー
    #[error("入力エラー: {0}")]
    Validation(String),

    /// データベース操作のエラー
    #[error("データベースエラー: {0}")]
    Database(String),

    /// ファイルI/Oエラー
    #[error("ファイルエラー: {0}")]
    Io(String),

    /// ONNXランタイムのエラー
    #[error("ONNXエラー: {0}")]
    Onnx(String),

    /// ComfyUI連携のエラー
    #[error("ComfyUIエラー: {0}")]
    ComfyUI(String),

    /// 画像処理のエラー
    #[error("画像処理エラー: {0}")]
    ImageProcessing(String),

    /// エクスポート処理のエラー
    #[error("エクスポートエラー: {0}")]
    Export(String),

    /// 予期しない内部エラー
    #[error("内部エラー: {0}")]
    Internal(String),
}

impl AppError {
    /// ユーザー向けのエラーコードを返す
    ///
    /// フロントエンド側でエラー種別による分岐に使用する．
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Validation(_) => "VALIDATION",
            AppError::Database(_) => "DATABASE",
            AppError::Io(_) => "IO",
            AppError::Onnx(_) => "ONNX",
            AppError::ComfyUI(_) => "COMFYUI",
            AppError::ImageProcessing(_) => "IMAGE_PROCESSING",
            AppError::Export(_) => "EXPORT",
            AppError::Internal(_) => "INTERNAL",
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Validation(err.to_string())
    }
}
