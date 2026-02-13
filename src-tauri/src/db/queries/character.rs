use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{Character, CreateCharacter, UpdateCharacter};

pub fn create_character(conn: &Connection, input: &CreateCharacter) -> Result<Character> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO characters (id, project_id, name, category, custom_prompt,
         status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, ?7)",
        params![
            id,
            input.project_id,
            input.name,
            input.category.as_deref().unwrap_or("enemy"),
            input.custom_prompt,
            now,
            now,
        ],
    )?;

    get_character(conn, &id)?.ok_or_else(|| anyhow::anyhow!("Failed to create character"))
}

pub fn get_character(conn: &Connection, id: &str) -> Result<Option<Character>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, category, custom_prompt,
                status, spritesheet_path, created_at, updated_at
         FROM characters WHERE id = ?1",
    )?;

    let character = stmt
        .query_row(params![id], |row| {
            Ok(Character {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                custom_prompt: row.get(4)?,
                status: row.get(5)?,
                spritesheet_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .optional()?;

    Ok(character)
}

pub fn list_characters_by_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<Character>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, category, custom_prompt,
                status, spritesheet_path, created_at, updated_at
         FROM characters WHERE project_id = ?1 ORDER BY created_at ASC",
    )?;

    let characters = stmt
        .query_map(params![project_id], |row| {
            Ok(Character {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                custom_prompt: row.get(4)?,
                status: row.get(5)?,
                spritesheet_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(characters)
}

pub fn update_character(
    conn: &Connection,
    id: &str,
    input: &UpdateCharacter,
) -> Result<Character> {
    let existing = get_character(conn, id)?
        .ok_or_else(|| anyhow::anyhow!("Character not found: {}", id))?;

    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE characters SET
            name = ?1, category = ?2, custom_prompt = ?3, updated_at = ?4
         WHERE id = ?5",
        params![
            input.name.as_deref().unwrap_or(&existing.name),
            input.category.as_deref().unwrap_or(&existing.category),
            input
                .custom_prompt
                .as_ref()
                .or(existing.custom_prompt.as_ref()),
            now,
            id,
        ],
    )?;

    get_character(conn, id)?
        .ok_or_else(|| anyhow::anyhow!("Character not found after update"))
}

pub fn update_character_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    let valid = ["draft", "importing", "processing", "complete"];
    if !valid.contains(&status) {
        return Err(anyhow::anyhow!("Invalid character status: {}", status));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE characters SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now, id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Character not found: {}", id));
    }
    Ok(())
}

pub fn delete_character(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM characters WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(anyhow::anyhow!("Character not found: {}", id));
    }
    Ok(())
}
