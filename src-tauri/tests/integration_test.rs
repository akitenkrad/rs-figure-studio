/// 統合テスト
///
/// DB レイヤー，ステータス遷移，ファイル名パーサー，
/// SpritesheetMeta 生成，ComfyUI なしパイプラインのテストを実施する．

use rusqlite::Connection;

// ============================================================
// テストヘルパー
// ============================================================

/// インメモリ SQLite DB を初期化して返す
fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

    // v1 マイグレーションを手動実行
    conn.execute_batch(include_str!("../migrations/v1.sql"))
        .expect("Failed to run v1 migration");

    conn
}

/// テスト用プロジェクトを作成して返す
fn create_test_project(conn: &Connection, base_path: &str) -> figurine_studio_lib::models::Project {
    use figurine_studio_lib::models::CreateProject;

    let input = CreateProject {
        name: "Test Project".into(),
        base_path: base_path.into(),
        tile_width: Some(64),
        tile_height: Some(64),
        directions: None,
        animations: None,
        style_prompt: None,
        negative_prompt: None,
        controlnet_weight: None,
        bevy_output_path: None,
    };

    figurine_studio_lib::db::queries::project::create_project(conn, &input)
        .expect("Failed to create test project")
}

/// テスト用キャラクターを作成して返す
fn create_test_character(
    conn: &Connection,
    project_id: &str,
    name: &str,
) -> figurine_studio_lib::models::Character {
    use figurine_studio_lib::models::CreateCharacter;

    let input = CreateCharacter {
        project_id: project_id.into(),
        name: name.into(),
        category: Some("enemy".into()),
        custom_prompt: None,
    };

    figurine_studio_lib::db::queries::character::create_character(conn, &input)
        .expect("Failed to create test character")
}

/// テスト用スプライトをバッチ作成して返す
fn create_test_sprites(
    conn: &Connection,
    character_id: &str,
    directions: &[&str],
    animations: &[(&str, u32)], // (name, frame_count)
) -> Vec<figurine_studio_lib::models::Sprite> {
    use figurine_studio_lib::models::CreateSprite;

    let mut inputs = Vec::new();
    for dir in directions {
        for (anim, frame_count) in animations {
            for frame in 0..*frame_count as i32 {
                inputs.push(CreateSprite {
                    character_id: character_id.into(),
                    direction: dir.to_string(),
                    animation: anim.to_string(),
                    frame_index: frame,
                    raw_path: Some(format!(
                        "/tmp/test/raw/warrior_{}_{}_{:02}.png",
                        dir, anim, frame
                    )),
                });
            }
        }
    }

    figurine_studio_lib::db::queries::sprite::create_sprites_batch(conn, &inputs)
        .expect("Failed to create test sprites")
}

// ============================================================
// 1. DB レイヤーテスト
// ============================================================

#[test]
fn test_db_full_data_flow() {
    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    // プロジェクト作成
    let project = create_test_project(&conn, &base_path);
    assert_eq!(project.name, "Test Project");
    assert_eq!(project.tile_width, 64);
    assert_eq!(project.tile_height, 64);
    assert_eq!(project.directions.len(), 4); // デフォルト: down, left, right, up
    assert_eq!(project.animations.len(), 4); // デフォルト: idle, walk, attack, hit

    // プロジェクト取得
    let fetched = figurine_studio_lib::db::queries::project::get_project(&conn, &project.id)
        .unwrap()
        .expect("Project should exist");
    assert_eq!(fetched.id, project.id);

    // キャラクター作成
    let character = create_test_character(&conn, &project.id, "warrior");
    assert_eq!(character.name, "warrior");
    assert_eq!(character.category, "enemy");
    assert_eq!(character.status, "draft");

    // キャラクター一覧
    let characters =
        figurine_studio_lib::db::queries::character::list_characters_by_project(&conn, &project.id)
            .unwrap();
    assert_eq!(characters.len(), 1);

    // スプライトバッチ作成
    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down", "left", "right", "up"],
        &[("idle", 2), ("walk", 4)],
    );
    // 4方向 * (2+4)フレーム = 24 スプライト
    assert_eq!(sprites.len(), 24);

    // スプライトクエリ
    let queried_sprites =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();
    assert_eq!(queried_sprites.len(), 24);

    // 全スプライトのステータスが raw であること
    for sprite in &queried_sprites {
        assert_eq!(sprite.status, "raw");
    }
}

#[test]
fn test_db_project_list_returns_empty_initially() {
    let conn = setup_test_db();
    let projects = figurine_studio_lib::db::queries::project::list_projects(&conn).unwrap();
    assert!(projects.is_empty());
}

#[test]
fn test_db_cascade_delete() {
    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "goblin");
    let _sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down"],
        &[("idle", 2)],
    );

    // スプライトが2件存在
    let sprites_before =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();
    assert_eq!(sprites_before.len(), 2);

    // プロジェクト削除 → CASCADE でキャラクター・スプライトも削除される
    figurine_studio_lib::db::queries::project::delete_project(&conn, &project.id).unwrap();

    // プロジェクトが存在しない
    let project_after =
        figurine_studio_lib::db::queries::project::get_project(&conn, &project.id).unwrap();
    assert!(project_after.is_none());

    // キャラクターも削除済み
    let char_after =
        figurine_studio_lib::db::queries::character::get_character(&conn, &character.id).unwrap();
    assert!(char_after.is_none());

    // スプライトも削除済み
    let sprites_after =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();
    assert!(sprites_after.is_empty());
}

#[test]
fn test_db_update_project() {
    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);

    let update = figurine_studio_lib::models::UpdateProject {
        name: Some("Updated Name".into()),
        base_path: None,
        tile_width: Some(128),
        tile_height: Some(128),
        directions: None,
        animations: None,
        style_prompt: None,
        negative_prompt: None,
        controlnet_weight: None,
        bevy_output_path: None,
    };

    let updated =
        figurine_studio_lib::db::queries::project::update_project(&conn, &project.id, &update)
            .unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.tile_width, 128);
    assert_eq!(updated.tile_height, 128);
    // directions は変更なし
    assert_eq!(updated.directions.len(), 4);
}

// ============================================================
// 2. ステータス遷移テスト
// ============================================================

#[test]
fn test_sprite_status_flow_raw_to_finalized() {
    use figurine_studio_lib::models::SpriteStatus;

    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "knight");
    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down"],
        &[("idle", 1)],
    );
    let sprite = &sprites[0];

    // 初期状態: raw
    assert_eq!(sprite.status, "raw");

    // raw → ai_processed
    figurine_studio_lib::db::queries::sprite::update_sprite_status(
        &conn,
        &sprite.id,
        &SpriteStatus::AiProcessed,
    )
    .unwrap();
    let updated = get_sprite_status(&conn, &sprite.id);
    assert_eq!(updated, "ai_processed");

    // ai_processed → bg_removed
    figurine_studio_lib::db::queries::sprite::update_sprite_status(
        &conn,
        &sprite.id,
        &SpriteStatus::BgRemoved,
    )
    .unwrap();
    let updated = get_sprite_status(&conn, &sprite.id);
    assert_eq!(updated, "bg_removed");

    // bg_removed → finalized
    figurine_studio_lib::db::queries::sprite::update_sprite_status(
        &conn,
        &sprite.id,
        &SpriteStatus::Finalized,
    )
    .unwrap();
    let updated = get_sprite_status(&conn, &sprite.id);
    assert_eq!(updated, "finalized");
}

#[test]
fn test_sprite_status_direct_to_finalized() {
    use figurine_studio_lib::models::SpriteStatus;

    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "slime");
    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["up"],
        &[("walk", 1)],
    );

    // ComfyUI なしパス: raw → bg_removed → finalized
    figurine_studio_lib::db::queries::sprite::update_sprite_status(
        &conn,
        &sprites[0].id,
        &SpriteStatus::BgRemoved,
    )
    .unwrap();

    figurine_studio_lib::db::queries::sprite::update_sprite_status(
        &conn,
        &sprites[0].id,
        &SpriteStatus::Finalized,
    )
    .unwrap();

    let status = get_sprite_status(&conn, &sprites[0].id);
    assert_eq!(status, "finalized");
}

#[test]
fn test_sprite_path_update() {
    use figurine_studio_lib::models::UpdateSpritePaths;

    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "mage");
    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down"],
        &[("idle", 1)],
    );

    // processed_path を設定
    figurine_studio_lib::db::queries::sprite::update_sprite_paths(
        &conn,
        &sprites[0].id,
        &UpdateSpritePaths {
            raw_path: None,
            processed_path: Some("/tmp/processed/mage_down_idle_00.png".into()),
            final_path: None,
        },
    )
    .unwrap();

    // final_path を設定
    figurine_studio_lib::db::queries::sprite::update_sprite_paths(
        &conn,
        &sprites[0].id,
        &UpdateSpritePaths {
            raw_path: None,
            processed_path: None,
            final_path: Some("/tmp/normalized/mage_down_idle_00.png".into()),
        },
    )
    .unwrap();

    // 全パスを検証
    let sprite = get_sprite_by_id(&conn, &sprites[0].id);
    assert!(sprite.raw_path.is_some());
    assert_eq!(
        sprite.processed_path.as_deref(),
        Some("/tmp/processed/mage_down_idle_00.png")
    );
    assert_eq!(
        sprite.final_path.as_deref(),
        Some("/tmp/normalized/mage_down_idle_00.png")
    );
}

#[test]
fn test_character_status_transitions() {
    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "archer");

    // draft → importing
    figurine_studio_lib::db::queries::character::update_character_status(
        &conn,
        &character.id,
        "importing",
    )
    .unwrap();

    let updated =
        figurine_studio_lib::db::queries::character::get_character(&conn, &character.id)
            .unwrap()
            .unwrap();
    assert_eq!(updated.status, "importing");

    // importing → processing
    figurine_studio_lib::db::queries::character::update_character_status(
        &conn,
        &character.id,
        "processing",
    )
    .unwrap();

    let updated =
        figurine_studio_lib::db::queries::character::get_character(&conn, &character.id)
            .unwrap()
            .unwrap();
    assert_eq!(updated.status, "processing");

    // processing → complete
    figurine_studio_lib::db::queries::character::update_character_status(
        &conn,
        &character.id,
        "complete",
    )
    .unwrap();

    let updated =
        figurine_studio_lib::db::queries::character::get_character(&conn, &character.id)
            .unwrap()
            .unwrap();
    assert_eq!(updated.status, "complete");
}

#[test]
fn test_character_invalid_status_rejected() {
    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "thief");

    let result = figurine_studio_lib::db::queries::character::update_character_status(
        &conn,
        &character.id,
        "invalid_status",
    );

    assert!(result.is_err());
}

// ============================================================
// 3. ファイル名パーサー統合テスト
// ============================================================

#[test]
fn test_filename_parser_multiple_filenames() {
    use figurine_studio_lib::services::filename_parser::FilenameParser;

    let parser = FilenameParser::new(
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
    );

    // 正常系: 各方向・アニメーションのバリエーション
    let test_cases = vec![
        ("warrior_down_idle_00.png", "warrior", "down", "idle", 0),
        ("warrior_up_walk_03.png", "warrior", "up", "walk", 3),
        ("dark_knight_left_attack_02.png", "dark_knight", "left", "attack", 2),
        ("Goblin-Right-Hit-01.png", "goblin", "right", "hit", 1),
        ("slime_down_idle.png", "slime", "down", "idle", 0), // フレーム番号なし
    ];

    for (filename, expected_char, expected_dir, expected_anim, expected_frame) in test_cases {
        let result = parser.parse(filename);
        assert!(
            result.is_some(),
            "Failed to parse: {}",
            filename
        );
        let parsed = result.unwrap();
        assert_eq!(parsed.character, expected_char, "Character mismatch for {}", filename);
        assert_eq!(parsed.direction, expected_dir, "Direction mismatch for {}", filename);
        assert_eq!(parsed.animation, expected_anim, "Animation mismatch for {}", filename);
        assert_eq!(parsed.frame_index, expected_frame, "Frame mismatch for {}", filename);
    }

    // 異常系
    assert!(parser.parse("invalid.png").is_none());
    assert!(parser.parse("warrior_diagonal_idle_00.png").is_none());
    assert!(parser.parse("").is_none());
}

#[test]
fn test_filename_parser_with_project_directions_and_animations() {
    use figurine_studio_lib::services::filename_parser::FilenameParser;

    // カスタム方向・アニメーション
    let parser = FilenameParser::new(
        vec!["north".into(), "south".into(), "east".into(), "west".into()],
        vec!["run".into(), "jump".into(), "die".into()],
    );

    let result = parser.parse("hero_north_run_05.png");
    assert!(result.is_some());
    let parsed = result.unwrap();
    assert_eq!(parsed.character, "hero");
    assert_eq!(parsed.direction, "north");
    assert_eq!(parsed.animation, "run");
    assert_eq!(parsed.frame_index, 5);

    // デフォルトの方向名は認識しない
    assert!(parser.parse("hero_down_run_00.png").is_none());
}

// ============================================================
// 4. SpritesheetMeta 生成テスト
// ============================================================

#[test]
fn test_spritesheet_meta_default_structure() {
    use figurine_studio_lib::commands::bevy_export::SpritesheetMeta;
    use figurine_studio_lib::models::AnimationDef;

    let directions = vec![
        "down".to_string(),
        "left".to_string(),
        "right".to_string(),
        "up".to_string(),
    ];

    let animations = vec![
        AnimationDef { name: "idle".into(), frame_count: 2, frame_duration_ms: 300 },
        AnimationDef { name: "walk".into(), frame_count: 4, frame_duration_ms: 150 },
        AnimationDef { name: "attack".into(), frame_count: 4, frame_duration_ms: 100 },
        AnimationDef { name: "hit".into(), frame_count: 2, frame_duration_ms: 200 },
    ];

    let meta = SpritesheetMeta::new("warrior", 64, 64, &directions, &animations);

    // バージョン
    assert_eq!(meta.version, "1.0");

    // キャラクター名
    assert_eq!(meta.character, "warrior");

    // スプライトシート情報
    assert_eq!(meta.spritesheet.columns, 12); // 2+4+4+2
    assert_eq!(meta.spritesheet.rows, 4);
    assert_eq!(meta.spritesheet.tile_width, 64);
    assert_eq!(meta.spritesheet.tile_height, 64);

    // 方向マッピング
    assert_eq!(meta.directions.len(), 4);
    assert_eq!(meta.directions[0].name, "down");
    assert_eq!(meta.directions[0].row, 0);
    assert_eq!(meta.directions[3].name, "up");
    assert_eq!(meta.directions[3].row, 3);

    // アニメーション定義
    assert_eq!(meta.animations.len(), 4);
    assert_eq!(meta.animations[0].name, "idle");
    assert_eq!(meta.animations[0].start_col, 0);
    assert_eq!(meta.animations[0].frame_count, 2);
    assert_eq!(meta.animations[0].frame_duration_ms, 300);

    assert_eq!(meta.animations[1].name, "walk");
    assert_eq!(meta.animations[1].start_col, 2); // idle(2)の後
    assert_eq!(meta.animations[1].frame_count, 4);

    assert_eq!(meta.animations[2].name, "attack");
    assert_eq!(meta.animations[2].start_col, 6); // idle(2)+walk(4)の後

    assert_eq!(meta.animations[3].name, "hit");
    assert_eq!(meta.animations[3].start_col, 10); // idle(2)+walk(4)+attack(4)の後
}

#[test]
fn test_spritesheet_meta_json_serialization() {
    use figurine_studio_lib::commands::bevy_export::SpritesheetMeta;
    use figurine_studio_lib::models::AnimationDef;

    let meta = SpritesheetMeta::new(
        "slime",
        32,
        32,
        &["down".into(), "up".into()],
        &[AnimationDef {
            name: "idle".into(),
            frame_count: 2,
            frame_duration_ms: 300,
        }],
    );

    let json = serde_json::to_value(&meta).expect("Failed to serialize meta");

    // JSON 構造が期待通り
    assert_eq!(json["version"], "1.0");
    assert_eq!(json["character"], "slime");
    assert_eq!(json["spritesheet"]["columns"], 2);
    assert_eq!(json["spritesheet"]["rows"], 2);
    assert_eq!(json["spritesheet"]["tile_width"], 32);
    assert_eq!(json["spritesheet"]["tile_height"], 32);
    assert_eq!(json["directions"][0]["name"], "down");
    assert_eq!(json["directions"][0]["row"], 0);
    assert_eq!(json["animations"][0]["name"], "idle");
    assert_eq!(json["animations"][0]["start_col"], 0);
    assert_eq!(json["animations"][0]["frame_count"], 2);
    assert_eq!(json["animations"][0]["frame_duration_ms"], 300);
}

#[test]
fn test_spritesheet_meta_single_direction_single_animation() {
    use figurine_studio_lib::commands::bevy_export::SpritesheetMeta;
    use figurine_studio_lib::models::AnimationDef;

    let meta = SpritesheetMeta::new(
        "npc",
        48,
        48,
        &["down".into()],
        &[AnimationDef {
            name: "idle".into(),
            frame_count: 1,
            frame_duration_ms: 500,
        }],
    );

    assert_eq!(meta.spritesheet.columns, 1);
    assert_eq!(meta.spritesheet.rows, 1);
    assert_eq!(meta.directions.len(), 1);
    assert_eq!(meta.animations.len(), 1);
}

// ============================================================
// 5. ComfyUI なしパス (パイプラインテスト)
// ============================================================

#[test]
fn test_pipeline_without_comfyui() {
    use figurine_studio_lib::models::{SpriteStatus, UpdateSpritePaths};

    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    // ===== Phase 1: プロジェクト & キャラクター作成 =====
    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "warrior");

    // ===== Phase 2: スプライトインポート =====
    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down", "left", "right", "up"],
        &[("idle", 2), ("walk", 4), ("attack", 4), ("hit", 2)],
    );
    // 4方向 * 12フレーム = 48 スプライト
    assert_eq!(sprites.len(), 48);

    // 全スプライトが raw ステータス
    for s in &sprites {
        assert_eq!(s.status, "raw");
        assert!(s.raw_path.is_some());
    }

    // ===== Phase 3: 背景除去 (ステータスのみシミュレーション) =====
    for sprite in &sprites {
        let bg_removed_path = format!(
            "{}/{}/bg_removed/{}_{}_{}_{:02}.png",
            base_path, character.name, character.name, sprite.direction, sprite.animation, sprite.frame_index
        );
        figurine_studio_lib::db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: Some(bg_removed_path),
                final_path: None,
            },
        )
        .unwrap();
        figurine_studio_lib::db::queries::sprite::update_sprite_status(
            &conn,
            &sprite.id,
            &SpriteStatus::BgRemoved,
        )
        .unwrap();
    }

    // ステータスが bg_removed に更新されていること
    let sprites_after_bg =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();
    for s in &sprites_after_bg {
        assert_eq!(s.status, "bg_removed");
        assert!(s.processed_path.is_some());
    }

    // ===== Phase 4: 正規化 → finalized (ステータスのみシミュレーション) =====
    for sprite in &sprites_after_bg {
        let normalized_path = format!(
            "{}/{}/normalized/{}_{}_{}_{:02}.png",
            base_path, character.name, character.name, sprite.direction, sprite.animation, sprite.frame_index
        );
        figurine_studio_lib::db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: None,
                final_path: Some(normalized_path),
            },
        )
        .unwrap();
        figurine_studio_lib::db::queries::sprite::update_sprite_status(
            &conn,
            &sprite.id,
            &SpriteStatus::Finalized,
        )
        .unwrap();
    }

    // 全スプライトが finalized
    let sprites_finalized =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();
    for s in &sprites_finalized {
        assert_eq!(s.status, "finalized");
        assert!(s.final_path.is_some());
    }

    // finalized スプライト数 = 48
    let finalized_count = sprites_finalized
        .iter()
        .filter(|s| s.status == "finalized" && s.final_path.is_some())
        .count();
    assert_eq!(finalized_count, 48);

    // ===== Phase 5: SpritesheetMeta 生成 =====
    use figurine_studio_lib::commands::bevy_export::SpritesheetMeta;

    let meta = SpritesheetMeta::new(
        &character.name,
        project.tile_width as u32,
        project.tile_height as u32,
        &project.directions,
        &project.animations,
    );

    assert_eq!(meta.character, "warrior");
    assert_eq!(meta.spritesheet.columns, 12);
    assert_eq!(meta.spritesheet.rows, 4);
    assert_eq!(meta.version, "1.0");

    // メタデータを JSON にシリアライズできること
    let meta_json = serde_json::to_string_pretty(&meta).unwrap();
    assert!(meta_json.contains("\"version\": \"1.0\""));
    assert!(meta_json.contains("\"character\": \"warrior\""));

    // ===== Phase 6: ファイル名サニタイズ検証 =====
    use figurine_studio_lib::services::path_utils;

    let safe_name = path_utils::sanitize_filename(&character.name).unwrap();
    assert_eq!(safe_name, "warrior");

    // 特殊文字を含む名前
    let safe_special = path_utils::sanitize_filename("Dark Knight v2").unwrap();
    assert_eq!(safe_special, "dark_knight_v2");
}

#[test]
fn test_pipeline_full_status_flow_with_comfyui_path() {
    use figurine_studio_lib::models::{SpriteStatus, UpdateSpritePaths};

    let conn = setup_test_db();
    let tmp = tempfile::tempdir().unwrap();
    let base_path = tmp.path().to_string_lossy().to_string();

    let project = create_test_project(&conn, &base_path);
    let character = create_test_character(&conn, &project.id, "dragon");

    let sprites = create_test_sprites(
        &conn,
        &character.id,
        &["down"],
        &[("idle", 2)],
    );

    // raw → ai_processed → bg_removed → finalized
    for sprite in &sprites {
        // Step 1: raw → ai_processed
        figurine_studio_lib::db::queries::sprite::update_sprite_status(
            &conn,
            &sprite.id,
            &SpriteStatus::AiProcessed,
        )
        .unwrap();
        figurine_studio_lib::db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: Some(format!("/tmp/ai/{}.png", sprite.id)),
                final_path: None,
            },
        )
        .unwrap();

        // Step 2: ai_processed → bg_removed
        figurine_studio_lib::db::queries::sprite::update_sprite_status(
            &conn,
            &sprite.id,
            &SpriteStatus::BgRemoved,
        )
        .unwrap();
        figurine_studio_lib::db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: Some(format!("/tmp/bg/{}.png", sprite.id)),
                final_path: None,
            },
        )
        .unwrap();

        // Step 3: bg_removed → finalized
        figurine_studio_lib::db::queries::sprite::update_sprite_status(
            &conn,
            &sprite.id,
            &SpriteStatus::Finalized,
        )
        .unwrap();
        figurine_studio_lib::db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: None,
                final_path: Some(format!("/tmp/final/{}.png", sprite.id)),
            },
        )
        .unwrap();
    }

    let final_sprites =
        figurine_studio_lib::db::queries::sprite::get_sprites_by_character(&conn, &character.id)
            .unwrap();

    for s in &final_sprites {
        assert_eq!(s.status, "finalized");
        assert!(s.final_path.is_some());
        assert!(s.processed_path.is_some());
        assert!(s.raw_path.is_some());
    }
}

// ============================================================
// 6. パスユーティリティテスト
// ============================================================

#[test]
fn test_path_utils_sanitize_and_export_flow() {
    use figurine_studio_lib::services::path_utils;

    let tmp = tempfile::tempdir().unwrap();
    let output_dir = tmp.path().join("bevy_assets").join("characters");

    // ディレクトリ作成
    path_utils::ensure_dir(&output_dir).unwrap();
    assert!(output_dir.exists());

    // ファイル名サニタイズ
    let names_and_expected = vec![
        ("warrior", "warrior"),
        ("Dark Knight", "dark_knight"),
        ("slime-v2", "slime-v2"),
        ("Fire_Mage", "fire_mage"),
        ("Test 123!", "test_123_"),
    ];

    for (input, expected) in names_and_expected {
        let result = path_utils::sanitize_filename(input).unwrap();
        assert_eq!(result, expected, "Mismatch for input: {}", input);
    }
}

#[test]
fn test_path_utils_traversal_prevention() {
    use figurine_studio_lib::services::path_utils;

    // 安全なパス
    assert!(path_utils::validate_user_path("characters/warrior").is_ok());
    assert!(path_utils::validate_user_path("warrior.png").is_ok());

    // 危険なパス
    assert!(path_utils::validate_user_path("../etc/passwd").is_err());
    assert!(path_utils::validate_user_path("/etc/passwd").is_err());
    assert!(path_utils::validate_user_path("characters/../../secret").is_err());
}

// ============================================================
// 7. Settings テスト
// ============================================================

#[test]
fn test_settings_crud() {
    let conn = setup_test_db();

    // デフォルト設定が存在
    let endpoint =
        figurine_studio_lib::db::queries::settings::get_setting(&conn, "comfyui_endpoint")
            .unwrap();
    assert_eq!(endpoint.as_deref(), Some("http://127.0.0.1:8188"));

    // 設定の更新
    figurine_studio_lib::db::queries::settings::set_setting(
        &conn,
        "comfyui_endpoint",
        "http://192.168.1.100:8188",
    )
    .unwrap();

    let updated =
        figurine_studio_lib::db::queries::settings::get_setting(&conn, "comfyui_endpoint")
            .unwrap();
    assert_eq!(updated.as_deref(), Some("http://192.168.1.100:8188"));

    // 存在しないキー
    let missing =
        figurine_studio_lib::db::queries::settings::get_setting(&conn, "nonexistent").unwrap();
    assert!(missing.is_none());
}

// ============================================================
// ヘルパー関数
// ============================================================

/// スプライトの現在のステータスを取得
fn get_sprite_status(conn: &Connection, sprite_id: &str) -> String {
    conn.query_row(
        "SELECT status FROM sprites WHERE id = ?1",
        rusqlite::params![sprite_id],
        |row| row.get(0),
    )
    .expect("Failed to get sprite status")
}

/// スプライトを ID で取得
fn get_sprite_by_id(
    conn: &Connection,
    sprite_id: &str,
) -> figurine_studio_lib::models::Sprite {
    let mut stmt = conn
        .prepare(
            "SELECT id, character_id, direction, animation, frame_index,
                    raw_path, processed_path, final_path, status, created_at
             FROM sprites WHERE id = ?1",
        )
        .unwrap();

    stmt.query_row(rusqlite::params![sprite_id], |row| {
        Ok(figurine_studio_lib::models::Sprite {
            id: row.get(0)?,
            character_id: row.get(1)?,
            direction: row.get(2)?,
            animation: row.get(3)?,
            frame_index: row.get(4)?,
            raw_path: row.get(5)?,
            processed_path: row.get(6)?,
            final_path: row.get(7)?,
            status: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .expect("Failed to get sprite by ID")
}
