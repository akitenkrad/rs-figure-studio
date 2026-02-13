use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::{CreateSprite, Sprite, SpriteStatus, UpdateSpritePaths};

pub fn create_sprite(conn: &Connection, input: &CreateSprite) -> Result<Sprite> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO sprites (id, character_id, direction, animation, frame_index,
         raw_path, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'raw', ?7)",
        params![
            id,
            input.character_id,
            input.direction,
            input.animation,
            input.frame_index,
            input.raw_path,
            now,
        ],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, character_id, direction, animation, frame_index,
                raw_path, processed_path, final_path, status, created_at
         FROM sprites WHERE id = ?1",
    )?;

    let sprite = stmt.query_row(params![id], |row| {
        Ok(Sprite {
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
    })?;

    Ok(sprite)
}

pub fn create_sprites_batch(conn: &Connection, inputs: &[CreateSprite]) -> Result<Vec<Sprite>> {
    let tx = conn.unchecked_transaction()?;
    let mut sprites = Vec::with_capacity(inputs.len());

    for input in inputs {
        let sprite = create_sprite(&tx, input)?;
        sprites.push(sprite);
    }

    tx.commit()?;
    Ok(sprites)
}

pub fn get_sprites_by_character(
    conn: &Connection,
    character_id: &str,
) -> Result<Vec<Sprite>> {
    let mut stmt = conn.prepare(
        "SELECT id, character_id, direction, animation, frame_index,
                raw_path, processed_path, final_path, status, created_at
         FROM sprites
         WHERE character_id = ?1
         ORDER BY direction, animation, frame_index",
    )?;

    let sprites = stmt
        .query_map(params![character_id], |row| {
            Ok(Sprite {
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
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sprites)
}

pub fn update_sprite_status(
    conn: &Connection,
    id: &str,
    status: &SpriteStatus,
) -> Result<()> {
    let affected = conn.execute(
        "UPDATE sprites SET status = ?1 WHERE id = ?2",
        params![status.as_str(), id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Sprite not found: {}", id));
    }
    Ok(())
}

pub fn update_sprite_paths(
    conn: &Connection,
    id: &str,
    paths: &UpdateSpritePaths,
) -> Result<()> {
    let mut sets = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref raw) = paths.raw_path {
        sets.push("raw_path = ?");
        values.push(Box::new(raw.clone()));
    }
    if let Some(ref processed) = paths.processed_path {
        sets.push("processed_path = ?");
        values.push(Box::new(processed.clone()));
    }
    if let Some(ref final_p) = paths.final_path {
        sets.push("final_path = ?");
        values.push(Box::new(final_p.clone()));
    }

    if sets.is_empty() {
        return Ok(());
    }

    values.push(Box::new(id.to_string()));
    let sql = format!("UPDATE sprites SET {} WHERE id = ?", sets.join(", "));

    let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let affected = conn.execute(&sql, params.as_slice())?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Sprite not found: {}", id));
    }
    Ok(())
}

pub fn update_sprite_assignment(
    conn: &Connection,
    id: &str,
    direction: &str,
    animation: &str,
) -> Result<()> {
    let affected = conn.execute(
        "UPDATE sprites SET direction = ?1, animation = ?2 WHERE id = ?3",
        params![direction, animation, id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Sprite not found: {}", id));
    }
    Ok(())
}
