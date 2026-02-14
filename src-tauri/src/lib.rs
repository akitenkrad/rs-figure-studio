pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod services;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir)?;

            let app_state = AppState::new(app_data_dir)?;
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Project
            commands::project_commands::create_project,
            commands::project_commands::get_project,
            commands::project_commands::list_projects,
            commands::project_commands::update_project,
            commands::project_commands::delete_project,
            // Character
            commands::character_commands::create_character,
            commands::character_commands::get_character,
            commands::character_commands::list_characters,
            commands::character_commands::update_character,
            commands::character_commands::delete_character,
            commands::character_commands::import_sprites,
            // Image Processing
            commands::image_processing::normalize_sprite,
            commands::image_processing::normalize_batch,
            commands::image_processing::normalize_palette,
            // Model Download
            commands::model_download::check_onnx_model,
            commands::model_download::download_onnx_model,
            // Background Removal
            commands::background_removal::initialize_onnx,
            commands::background_removal::remove_background,
            commands::background_removal::remove_background_batch,
            // Spritesheet
            commands::spritesheet::generate_spritesheet,
            // Bevy Export
            commands::bevy_export::export_for_bevy,
            // ComfyUI
            commands::comfyui::check_comfyui_connection,
            commands::comfyui::list_workflows,
            commands::comfyui::create_workflow,
            commands::comfyui::import_workflow,
            commands::comfyui::update_workflow,
            commands::comfyui::delete_workflow,
            commands::comfyui::set_default_workflow,
            commands::comfyui::process_batch_comfyui,
            // ComfyUI Model
            commands::comfyui_model::check_comfyui_model,
            commands::comfyui_model::download_comfyui_model,
            commands::comfyui_model::get_comfyui_checkpoints_path,
            // Settings
            commands::settings_commands::get_settings,
            commands::settings_commands::update_setting,
            // Sprite
            commands::sprite_commands::list_sprites,
            commands::sprite_commands::update_sprite_assignment,
            // Generation (AI character generation)
            commands::generation::generate_concept,
            commands::generation::generate_concept_art,
            commands::generation::convert_to_pixel_art,
            commands::generation::generate_directions,
            commands::generation::generate_animation_frames,
            commands::generation::promote_generated_to_raw,
            commands::generation::test_cloud_api_connection,
            commands::generation::get_generation_state,
            // LoRA
            commands::lora::import_lora,
            commands::lora::list_loras,
            commands::lora::delete_lora,
            commands::lora::assign_lora,
            commands::lora::unassign_lora,
            commands::lora::list_character_loras,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
