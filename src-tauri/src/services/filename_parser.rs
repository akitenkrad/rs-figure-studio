/// ファイル名パーサー
///
/// MagicaVoxelなどから出力されたスプライト画像のファイル名を解析し，
/// キャラクター名，方向，アニメーション名，フレーム番号を抽出する．
///
/// 推奨命名パターン: `{character}_{direction}_{animation}_{frame:02d}.png`

/// パース結果
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFilename {
    pub character: String,
    pub direction: String,
    pub animation: String,
    pub frame_index: u32,
}

/// ファイル名パーサー
///
/// プロジェクトで定義された有効な方向名・アニメーション名に基づいて，
/// ファイル名から各フィールドを抽出する．
pub struct FilenameParser {
    /// プロジェクトで定義された有効な方向名
    valid_directions: Vec<String>,
    /// プロジェクトで定義された有効なアニメーション名
    valid_animations: Vec<String>,
}

impl FilenameParser {
    pub fn new(directions: Vec<String>, animations: Vec<String>) -> Self {
        Self {
            valid_directions: directions,
            valid_animations: animations,
        }
    }

    /// ファイル名をパースして character, direction, animation, frame_index を抽出
    ///
    /// 対応するバリエーション:
    /// - アンダースコア区切り: `warrior_down_walk_01.png`
    /// - ハイフン区切り: `warrior-down-walk-01.png`
    /// - ゼロ埋めなし: `warrior_down_walk_1.png`
    /// - 大文字混在: `Warrior_Down_Walk_01.png`（小文字に正規化）
    /// - フレーム番号なし: `warrior_down_idle.png`（デフォルト0）
    pub fn parse(&self, filename: &str) -> Option<ParsedFilename> {
        // 拡張子を除去
        let stem = filename
            .strip_suffix(".png")
            .or_else(|| filename.strip_suffix(".jpg"))
            .or_else(|| filename.strip_suffix(".jpeg"))
            .or_else(|| filename.strip_suffix(".webp"))
            .or_else(|| filename.strip_suffix(".bmp"))
            .or_else(|| filename.strip_suffix(".tiff"))
            .or_else(|| filename.strip_suffix(".tif"))
            .unwrap_or(filename);

        // 小文字に正規化
        let normalized = stem.to_lowercase();

        // 区切り文字で分割（アンダースコア or ハイフン）
        let parts: Vec<&str> = normalized
            .split(|c: char| c == '_' || c == '-')
            .collect();

        if parts.len() < 3 {
            return None;
        }

        // 末尾からフレーム番号を探索
        let frame_index: u32;
        let remaining_parts: &[&str];

        if let Ok(idx) = parts.last()?.parse::<u32>() {
            frame_index = idx;
            remaining_parts = &parts[..parts.len() - 1];
        } else {
            // フレーム番号がない場合はデフォルト0
            frame_index = 0;
            remaining_parts = &parts;
        }

        // direction と animation を前方から探索
        let mut direction: Option<String> = None;
        let mut animation: Option<String> = None;
        let mut char_end_idx: usize = 0;

        for (i, part) in remaining_parts.iter().enumerate() {
            if direction.is_none()
                && self.valid_directions.iter().any(|d| d == part)
            {
                direction = Some(part.to_string());
                char_end_idx = i;
            } else if direction.is_some()
                && animation.is_none()
                && self.valid_animations.iter().any(|a| a == part)
            {
                animation = Some(part.to_string());
            }
        }

        let direction = direction?;
        let animation = animation?;

        // direction より前の部分がキャラクター名
        let character = remaining_parts[..char_end_idx].join("_");
        if character.is_empty() {
            return None;
        }

        Some(ParsedFilename {
            character,
            direction,
            animation,
            frame_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_parser() -> FilenameParser {
        FilenameParser::new(
            vec![
                "down".into(),
                "left".into(),
                "right".into(),
                "up".into(),
            ],
            vec![
                "idle".into(),
                "walk".into(),
                "attack".into(),
                "hit".into(),
            ],
        )
    }

    // ===== 正常系テスト =====

    #[test]
    fn test_standard_filename() {
        let parser = default_parser();
        let result = parser.parse("warrior_down_idle_00.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "warrior".into(),
                direction: "down".into(),
                animation: "idle".into(),
                frame_index: 0,
            })
        );
    }

    #[test]
    fn test_multi_word_character_name() {
        let parser = default_parser();
        let result = parser.parse("dark_knight_left_walk_03.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "dark_knight".into(),
                direction: "left".into(),
                animation: "walk".into(),
                frame_index: 3,
            })
        );
    }

    #[test]
    fn test_hyphen_separator() {
        let parser = default_parser();
        let result = parser.parse("Goblin-Right-Hit-00.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "goblin".into(),
                direction: "right".into(),
                animation: "hit".into(),
                frame_index: 0,
            })
        );
    }

    #[test]
    fn test_case_insensitive() {
        let parser = default_parser();
        let result = parser.parse("Warrior_Down_Walk_01.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "warrior".into(),
                direction: "down".into(),
                animation: "walk".into(),
                frame_index: 1,
            })
        );
    }

    #[test]
    fn test_no_frame_number() {
        let parser = default_parser();
        let result = parser.parse("warrior_down_idle.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "warrior".into(),
                direction: "down".into(),
                animation: "idle".into(),
                frame_index: 0,
            })
        );
    }

    #[test]
    fn test_up_direction_attack_animation() {
        let parser = default_parser();
        let result = parser.parse("slime_up_attack_01.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "slime".into(),
                direction: "up".into(),
                animation: "attack".into(),
                frame_index: 1,
            })
        );
    }

    #[test]
    fn test_no_zero_padding() {
        let parser = default_parser();
        let result = parser.parse("warrior_down_walk_1.png");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "warrior".into(),
                direction: "down".into(),
                animation: "walk".into(),
                frame_index: 1,
            })
        );
    }

    #[test]
    fn test_jpg_extension() {
        let parser = default_parser();
        let result = parser.parse("warrior_down_idle_02.jpg");
        assert_eq!(
            result,
            Some(ParsedFilename {
                character: "warrior".into(),
                direction: "down".into(),
                animation: "idle".into(),
                frame_index: 2,
            })
        );
    }

    // ===== 異常系テスト =====

    #[test]
    fn test_too_few_parts() {
        let parser = default_parser();
        let result = parser.parse("warrior_down.png");
        assert_eq!(result, None);
    }

    #[test]
    fn test_invalid_direction() {
        let parser = default_parser();
        let result = parser.parse("warrior_diagonal_idle_00.png");
        assert_eq!(result, None);
    }

    #[test]
    fn test_invalid_animation() {
        let parser = default_parser();
        let result = parser.parse("warrior_down_dance_00.png");
        assert_eq!(result, None);
    }

    #[test]
    fn test_empty_filename() {
        let parser = default_parser();
        let result = parser.parse("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_no_character_name() {
        // direction が最初のパートだとキャラクター名が空になる
        let parser = default_parser();
        let result = parser.parse("down_idle_00.png");
        assert_eq!(result, None);
    }
}
