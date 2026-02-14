use rusqlite::{params, Connection};

/// Current schema version
const CURRENT_VERSION: i32 = 3;

/// Migration function type
type MigrationFn = fn(&Connection) -> anyhow::Result<()>;

/// Migration definition
struct Migration {
    version: i32,
    description: &'static str,
    up: MigrationFn,
}

/// List of all migrations
fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial schema: projects, characters, sprites, workflows, jobs, settings",
            up: migrate_v1,
        },
        Migration {
            version: 2,
            description: "Add generated status support and concept_image_path to characters",
            up: migrate_v2,
        },
        Migration {
            version: 3,
            description: "Cloud API support, LoRA management tables",
            up: migrate_v3,
        },
    ]
}

/// Run all pending migrations
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Create schema_version table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now')),
            description TEXT
        );",
    )?;

    // Get current version
    let current: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Run pending migrations
    let all_migrations = migrations();
    assert!(
        all_migrations.last().map_or(true, |m| m.version == CURRENT_VERSION),
        "CURRENT_VERSION ({}) does not match last migration version",
        CURRENT_VERSION,
    );

    for migration in all_migrations {
        if migration.version > current {
            log::info!(
                "Applying migration v{}: {}",
                migration.version,
                migration.description
            );
            (migration.up)(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, description) VALUES (?1, ?2)",
                params![migration.version, migration.description],
            )?;
        }
    }

    Ok(())
}

/// Migration v1: Initial schema
fn migrate_v1(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v1.sql"))?;
    Ok(())
}

/// Migration v2: Add generated status support and concept_image_path
fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v2.sql"))?;
    Ok(())
}

/// Migration v3: Cloud API support, LoRA management tables
fn migrate_v3(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(include_str!("../../migrations/v3.sql"))?;
    Ok(())
}
