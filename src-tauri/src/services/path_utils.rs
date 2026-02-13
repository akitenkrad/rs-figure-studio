/// ファイルパスのサニタイズ・検証ユーティリティ
///
/// ファイル名の安全性確保，ディレクトリ自動作成，
/// パストラバーサル攻撃の防止を提供する．

use std::path::Path;

use crate::error::AppError;

/// ファイル名を安全な文字列に変換する
///
/// 英数字・アンダースコア・ハイフン以外の文字をアンダースコアに置換し，
/// 小文字に正規化する．空文字列の場合はエラーを返す．
pub fn sanitize_filename(name: &str) -> Result<String, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "ファイル名が空です".into(),
        ));
    }

    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase();

    // 全てアンダースコアになった場合はエラー
    if sanitized.chars().all(|c| c == '_' || c == '-') {
        return Err(AppError::Validation(format!(
            "ファイル名に有効な英数字が含まれていません: {}",
            name
        )));
    }

    Ok(sanitized)
}

/// ディレクトリが存在しなければ作成する
///
/// 中間ディレクトリも含めて再帰的に作成する．
/// 既に存在する場合は何もしない（冪等）．
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| {
            AppError::Io(format!(
                "ディレクトリの作成に失敗しました: {} ({})",
                path.display(),
                e
            ))
        })?;
    }
    Ok(())
}

/// ユーザー入力パスのパストラバーサル検証
///
/// `..` セグメントや絶対パスを含む入力を拒否し，
/// ディレクトリトラバーサル攻撃を防止する．
///
/// # Returns
/// - `Ok(())` — パスが安全な場合
/// - `Err(AppError::Validation)` — 危険なパスの場合
pub fn validate_user_path(input: &str) -> Result<(), AppError> {
    let path = Path::new(input);

    // 絶対パスを拒否（ユーザー入力はベースパスからの相対であるべき）
    if path.is_absolute() {
        return Err(AppError::Validation(
            "絶対パスは許可されていません".into(),
        ));
    }

    // ".." セグメントを拒否
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(AppError::Validation(
                "パスに '..' を含めることはできません".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== sanitize_filename テスト =====

    #[test]
    fn test_sanitize_simple_name() {
        assert_eq!(sanitize_filename("warrior").unwrap(), "warrior");
    }

    #[test]
    fn test_sanitize_spaces_replaced() {
        assert_eq!(sanitize_filename("Dark Knight").unwrap(), "dark_knight");
    }

    #[test]
    fn test_sanitize_special_chars() {
        assert_eq!(
            sanitize_filename("warrior/slayer\\v2").unwrap(),
            "warrior_slayer_v2"
        );
    }

    #[test]
    fn test_sanitize_mixed_case() {
        assert_eq!(sanitize_filename("DarkKnight").unwrap(), "darkknight");
    }

    #[test]
    fn test_sanitize_hyphens_preserved() {
        assert_eq!(sanitize_filename("dark-knight").unwrap(), "dark-knight");
    }

    #[test]
    fn test_sanitize_empty_name_error() {
        assert!(sanitize_filename("").is_err());
        assert!(sanitize_filename("   ").is_err());
    }

    #[test]
    fn test_sanitize_all_special_chars_error() {
        assert!(sanitize_filename("///").is_err());
    }

    // ===== ensure_dir テスト =====

    #[test]
    fn test_ensure_dir_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let new_dir = tmp.path().join("a").join("b").join("c");
        assert!(!new_dir.exists());

        ensure_dir(&new_dir).unwrap();
        assert!(new_dir.exists());
    }

    #[test]
    fn test_ensure_dir_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("existing");
        std::fs::create_dir_all(&dir).unwrap();

        // 2回目もエラーにならない
        ensure_dir(&dir).unwrap();
        assert!(dir.exists());
    }

    // ===== validate_user_path テスト =====

    #[test]
    fn test_validate_safe_relative_path() {
        assert!(validate_user_path("characters/warrior").is_ok());
    }

    #[test]
    fn test_validate_simple_name() {
        assert!(validate_user_path("warrior.png").is_ok());
    }

    #[test]
    fn test_validate_rejects_parent_dir() {
        assert!(validate_user_path("../etc/passwd").is_err());
        assert!(validate_user_path("characters/../../secret").is_err());
    }

    #[test]
    fn test_validate_rejects_absolute_path() {
        assert!(validate_user_path("/etc/passwd").is_err());
    }
}
