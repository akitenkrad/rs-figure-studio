use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{CreateWorkflow, Workflow};

/// ワークフロー作成
///
/// comfyui_workflows テーブルに新しいワークフローを挿入する．
/// is_default が true の場合，同プロジェクト内の既存デフォルトを解除してから設定する．
pub fn create_workflow(conn: &Connection, input: &CreateWorkflow) -> Result<Workflow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let is_default = input.is_default.unwrap_or(false);

    let workflow_json_str = serde_json::to_string(&input.workflow_json)?;

    // is_default が true の場合，既存デフォルトを解除
    if is_default {
        conn.execute(
            "UPDATE comfyui_workflows SET is_default = 0
             WHERE project_id = ?1 AND is_default = 1",
            params![input.project_id],
        )?;
    }

    conn.execute(
        "INSERT INTO comfyui_workflows (id, project_id, name, workflow_json, is_default, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, input.project_id, input.name, workflow_json_str, is_default, now],
    )?;

    get_workflow(conn, &id)?.ok_or_else(|| anyhow::anyhow!("Failed to create workflow"))
}

/// ワークフロー取得（ID指定）
pub fn get_workflow(conn: &Connection, id: &str) -> Result<Option<Workflow>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, workflow_json, is_default, created_at
         FROM comfyui_workflows WHERE id = ?1",
    )?;

    let workflow = stmt
        .query_row(params![id], |row| {
            let workflow_json_str: String = row.get(3)?;
            let is_default_int: i32 = row.get(4)?;
            Ok(Workflow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                workflow_json: serde_json::from_str(&workflow_json_str).unwrap_or_default(),
                is_default: is_default_int != 0,
                created_at: row.get(5)?,
            })
        })
        .optional()?;

    Ok(workflow)
}

/// 全ワークフロー一覧取得
pub fn list_all_workflows(conn: &Connection) -> Result<Vec<Workflow>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, workflow_json, is_default, created_at
         FROM comfyui_workflows
         ORDER BY is_default DESC, created_at DESC",
    )?;

    let workflows = stmt
        .query_map([], |row| {
            let workflow_json_str: String = row.get(3)?;
            let is_default_int: i32 = row.get(4)?;
            Ok(Workflow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                workflow_json: serde_json::from_str(&workflow_json_str).unwrap_or_default(),
                is_default: is_default_int != 0,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(workflows)
}

/// プロジェクトに紐付くワークフロー一覧取得
pub fn list_workflows(conn: &Connection, project_id: &str) -> Result<Vec<Workflow>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, workflow_json, is_default, created_at
         FROM comfyui_workflows
         WHERE project_id = ?1
         ORDER BY is_default DESC, created_at DESC",
    )?;

    let workflows = stmt
        .query_map(params![project_id], |row| {
            let workflow_json_str: String = row.get(3)?;
            let is_default_int: i32 = row.get(4)?;
            Ok(Workflow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                workflow_json: serde_json::from_str(&workflow_json_str).unwrap_or_default(),
                is_default: is_default_int != 0,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(workflows)
}

/// ワークフロー更新（名前・JSONの変更）
pub fn update_workflow(
    conn: &Connection,
    id: &str,
    name: &str,
    workflow_json: &serde_json::Value,
) -> Result<()> {
    let workflow_json_str = serde_json::to_string(workflow_json)?;

    let affected = conn.execute(
        "UPDATE comfyui_workflows SET name = ?1, workflow_json = ?2 WHERE id = ?3",
        params![name, workflow_json_str, id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Workflow not found: {}", id));
    }
    Ok(())
}

/// ワークフロー削除
pub fn delete_workflow(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "DELETE FROM comfyui_workflows WHERE id = ?1",
        params![id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Workflow not found: {}", id));
    }
    Ok(())
}

/// デフォルトワークフロー設定
///
/// 同プロジェクト内の既存デフォルトを解除し，指定ワークフローをデフォルトに設定する．
/// トランザクション内で実行される．
pub fn set_default_workflow(conn: &Connection, id: &str, project_id: &str) -> Result<()> {
    let tx = conn.unchecked_transaction()?;

    // 既存のデフォルトを解除
    tx.execute(
        "UPDATE comfyui_workflows SET is_default = 0
         WHERE project_id = ?1 AND is_default = 1",
        params![project_id],
    )?;

    // 新しいデフォルトを設定
    let affected = tx.execute(
        "UPDATE comfyui_workflows SET is_default = 1
         WHERE id = ?1 AND project_id = ?2",
        params![id, project_id],
    )?;

    if affected == 0 {
        return Err(anyhow::anyhow!("Workflow not found: {} (project: {})", id, project_id));
    }

    tx.commit()?;
    Ok(())
}

/// デフォルトワークフロー取得
pub fn get_default_workflow(conn: &Connection, project_id: &str) -> Result<Option<Workflow>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, workflow_json, is_default, created_at
         FROM comfyui_workflows
         WHERE project_id = ?1 AND is_default = 1",
    )?;

    let workflow = stmt
        .query_row(params![project_id], |row| {
            let workflow_json_str: String = row.get(3)?;
            let is_default_int: i32 = row.get(4)?;
            Ok(Workflow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                workflow_json: serde_json::from_str(&workflow_json_str).unwrap_or_default(),
                is_default: is_default_int != 0,
                created_at: row.get(5)?,
            })
        })
        .optional()?;

    Ok(workflow)
}
