use crate::db;
use crate::error::AppError;
use crate::models::{CreateProject, Project, UpdateProject};
use crate::state::AppState;

#[tauri::command]
pub async fn create_project(
    state: tauri::State<'_, AppState>,
    input: CreateProject,
) -> Result<Project, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Validation
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Project name is required".into()));
    }

    // Create base_path directory
    std::fs::create_dir_all(&input.base_path)
        .map_err(|e| AppError::Io(format!("Failed to create base_path: {}", e)))?;

    let project = db::queries::project::create_project(&conn, &input)?;
    Ok(project)
}

#[tauri::command]
pub async fn get_project(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Project, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::project::get_project(&conn, &id)?
        .ok_or_else(|| AppError::NotFound(format!("Project not found: {}", id)))
}

#[tauri::command]
pub async fn list_projects(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Project>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let projects = db::queries::project::list_projects(&conn)?;
    Ok(projects)
}

#[tauri::command]
pub async fn update_project(
    state: tauri::State<'_, AppState>,
    id: String,
    input: UpdateProject,
) -> Result<Project, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let project = db::queries::project::update_project(&conn, &id, &input)?;
    Ok(project)
}

#[tauri::command]
pub async fn delete_project(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Get project info for file deletion
    let project = db::queries::project::get_project(&conn, &id)?
        .ok_or_else(|| AppError::NotFound(format!("Project not found: {}", id)))?;

    // DB delete (CASCADE)
    db::queries::project::delete_project(&conn, &id)?;

    // Best-effort file system cleanup
    if let Err(e) = std::fs::remove_dir_all(&project.base_path) {
        log::warn!("Failed to remove project directory: {}", e);
    }

    Ok(())
}
