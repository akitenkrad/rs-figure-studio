use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// ============================================================
// LoRA Model Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraModel {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoraModel {
    pub name: String,
    pub file_path: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterLora {
    pub character_id: String,
    pub lora_id: String,
    pub lora_name: String,
    pub lora_file_path: String,
    pub weight: f64,
}

// ============================================================
// LoRA CRUD
// ============================================================

pub fn create_lora(conn: &Connection, input: &CreateLoraModel) -> Result<LoraModel> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO lora_models (id, name, file_path, description, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            input.name,
            input.file_path,
            input.description.as_deref().unwrap_or(""),
            now,
        ],
    )?;

    get_lora(conn, &id)?.ok_or_else(|| anyhow::anyhow!("Failed to create LoRA model"))
}

pub fn get_lora(conn: &Connection, id: &str) -> Result<Option<LoraModel>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, file_path, description, created_at
         FROM lora_models WHERE id = ?1",
    )?;

    let lora = stmt
        .query_row(params![id], |row| {
            Ok(LoraModel {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .optional()?;

    Ok(lora)
}

pub fn list_loras(conn: &Connection) -> Result<Vec<LoraModel>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, file_path, description, created_at
         FROM lora_models ORDER BY created_at ASC",
    )?;

    let loras = stmt
        .query_map([], |row| {
            Ok(LoraModel {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(loras)
}

pub fn delete_lora(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM lora_models WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(anyhow::anyhow!("LoRA model not found: {}", id));
    }
    Ok(())
}

// ============================================================
// Character-LoRA Assignment
// ============================================================

pub fn assign_lora(
    conn: &Connection,
    character_id: &str,
    lora_id: &str,
    weight: f64,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO character_loras (character_id, lora_id, weight)
         VALUES (?1, ?2, ?3)",
        params![character_id, lora_id, weight],
    )?;
    Ok(())
}

pub fn unassign_lora(conn: &Connection, character_id: &str, lora_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM character_loras WHERE character_id = ?1 AND lora_id = ?2",
        params![character_id, lora_id],
    )?;
    Ok(())
}

pub fn list_character_loras(conn: &Connection, character_id: &str) -> Result<Vec<CharacterLora>> {
    let mut stmt = conn.prepare(
        "SELECT cl.character_id, cl.lora_id, lm.name, lm.file_path, cl.weight
         FROM character_loras cl
         JOIN lora_models lm ON lm.id = cl.lora_id
         WHERE cl.character_id = ?1
         ORDER BY lm.name ASC",
    )?;

    let assignments = stmt
        .query_map(params![character_id], |row| {
            Ok(CharacterLora {
                character_id: row.get(0)?,
                lora_id: row.get(1)?,
                lora_name: row.get(2)?,
                lora_file_path: row.get(3)?,
                weight: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(assignments)
}
