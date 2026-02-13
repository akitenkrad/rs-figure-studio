pub mod queries;
pub mod schema;

use rusqlite::Connection;

pub fn initialize(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")?;

    schema::run_migrations(conn)?;
    Ok(())
}
