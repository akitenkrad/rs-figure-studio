use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{AnimationDef, CreateProject, Project, UpdateProject};

pub fn create_project(conn: &Connection, input: &CreateProject) -> Result<Project> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let directions_json = serde_json::to_string(input.directions.as_ref().unwrap_or(&vec![
        "down".into(),
        "left".into(),
        "right".into(),
        "up".into(),
    ]))?;

    let animations_json = serde_json::to_string(input.animations.as_ref().unwrap_or(&vec![
        AnimationDef {
            name: "idle".into(),
            frame_count: 2,
            frame_duration_ms: 300,
        },
        AnimationDef {
            name: "walk".into(),
            frame_count: 4,
            frame_duration_ms: 150,
        },
        AnimationDef {
            name: "attack".into(),
            frame_count: 4,
            frame_duration_ms: 100,
        },
        AnimationDef {
            name: "hit".into(),
            frame_count: 2,
            frame_duration_ms: 200,
        },
    ]))?;

    conn.execute(
        "INSERT INTO projects (id, name, base_path, tile_width, tile_height,
         directions, animations, style_prompt, negative_prompt,
         controlnet_weight, bevy_output_path, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            input.name,
            input.base_path,
            input.tile_width.unwrap_or(64),
            input.tile_height.unwrap_or(64),
            directions_json,
            animations_json,
            input.style_prompt.as_deref().unwrap_or(
                "painted miniature figurine, soft PBR material, warm studio lighting, tabletop game piece"
            ),
            input.negative_prompt.as_deref().unwrap_or(
                "background, shadow on floor, realistic human"
            ),
            input.controlnet_weight.unwrap_or(0.8),
            input.bevy_output_path,
            now,
            now,
        ],
    )?;

    get_project(conn, &id)?.ok_or_else(|| anyhow::anyhow!("Failed to create project"))
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, base_path, tile_width, tile_height,
                directions, animations, style_prompt, negative_prompt,
                controlnet_weight, bevy_output_path, created_at, updated_at
         FROM projects WHERE id = ?1",
    )?;

    let project = stmt
        .query_row(params![id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                base_path: row.get(2)?,
                tile_width: row.get(3)?,
                tile_height: row.get(4)?,
                directions: serde_json::from_str(&row.get::<_, String>(5)?)
                    .unwrap_or_default(),
                animations: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or_default(),
                style_prompt: row.get(7)?,
                negative_prompt: row.get(8)?,
                controlnet_weight: row.get(9)?,
                bevy_output_path: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .optional()?;

    Ok(project)
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, base_path, tile_width, tile_height,
                directions, animations, style_prompt, negative_prompt,
                controlnet_weight, bevy_output_path, created_at, updated_at
         FROM projects ORDER BY updated_at DESC",
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                base_path: row.get(2)?,
                tile_width: row.get(3)?,
                tile_height: row.get(4)?,
                directions: serde_json::from_str(&row.get::<_, String>(5)?)
                    .unwrap_or_default(),
                animations: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or_default(),
                style_prompt: row.get(7)?,
                negative_prompt: row.get(8)?,
                controlnet_weight: row.get(9)?,
                bevy_output_path: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(projects)
}

pub fn update_project(conn: &Connection, id: &str, input: &UpdateProject) -> Result<Project> {
    let existing =
        get_project(conn, id)?.ok_or_else(|| anyhow::anyhow!("Project not found: {}", id))?;

    let now = chrono::Utc::now().to_rfc3339();

    let directions_json = match &input.directions {
        Some(d) => serde_json::to_string(d)?,
        None => serde_json::to_string(&existing.directions)?,
    };

    let animations_json = match &input.animations {
        Some(a) => serde_json::to_string(a)?,
        None => serde_json::to_string(&existing.animations)?,
    };

    conn.execute(
        "UPDATE projects SET
            name = ?1, base_path = ?2, tile_width = ?3, tile_height = ?4,
            directions = ?5, animations = ?6, style_prompt = ?7, negative_prompt = ?8,
            controlnet_weight = ?9, bevy_output_path = ?10, updated_at = ?11
         WHERE id = ?12",
        params![
            input.name.as_deref().unwrap_or(&existing.name),
            input.base_path.as_deref().unwrap_or(&existing.base_path),
            input.tile_width.unwrap_or(existing.tile_width),
            input.tile_height.unwrap_or(existing.tile_height),
            directions_json,
            animations_json,
            input
                .style_prompt
                .as_deref()
                .unwrap_or(&existing.style_prompt),
            input
                .negative_prompt
                .as_deref()
                .unwrap_or(&existing.negative_prompt),
            input
                .controlnet_weight
                .unwrap_or(existing.controlnet_weight),
            input
                .bevy_output_path
                .as_ref()
                .or(existing.bevy_output_path.as_ref()),
            now,
            id,
        ],
    )?;

    get_project(conn, id)?.ok_or_else(|| anyhow::anyhow!("Project not found after update"))
}

pub fn delete_project(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(anyhow::anyhow!("Project not found: {}", id));
    }
    // CASCADE DELETE removes characters, sprites, workflows, jobs automatically
    Ok(())
}
