//! Find-or-create access to the `project` table, keyed by the unique `root_path`.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::types::unix_timestamp;

/// Looks a project up without creating it.
///
/// [`find_or_create_project`] is the write path; a read-only command such as the CLI's
/// `history` must not leave a `project` row behind merely because someone asked about a
/// directory that has never been exported.
pub fn find_project_id(conn: &Connection, root_path: &str) -> Result<Option<i64>> {
    let id = conn
        .query_row(
            "SELECT id FROM project WHERE root_path = ?1",
            params![root_path],
            |row| row.get(0),
        )
        .optional()?;
    Ok(id)
}

/// Returns the existing `project.id` for `root_path` if one exists, otherwise inserts
/// a new row and returns its id. Never updates `name`/`primary_stack` on an existing
/// row — a project that already exists keeps whatever it was first created with.
pub fn find_or_create_project(
    conn: &Connection,
    root_path: &str,
    name: &str,
    primary_stack: Option<&str>,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM project WHERE root_path = ?1",
            params![root_path],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }

    conn.execute(
        "INSERT INTO project (root_path, name, primary_stack, created_at, last_export_at)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![root_path, name, primary_stack, unix_timestamp()],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::open;

    #[test]
    fn creates_once_and_finds_thereafter() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("codepack.db")).unwrap();

        let first = find_or_create_project(&conn, "/repo/one", "One", Some("rust")).unwrap();
        let second = find_or_create_project(&conn, "/repo/one", "Renamed", Some("python")).unwrap();
        assert_eq!(first, second);

        let name: String = conn
            .query_row(
                "SELECT name FROM project WHERE id = ?1",
                params![first],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            name, "One",
            "existing project row is never updated in place"
        );
    }

    #[test]
    fn distinct_roots_get_distinct_projects() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("codepack.db")).unwrap();

        let first = find_or_create_project(&conn, "/repo/one", "One", None).unwrap();
        let second = find_or_create_project(&conn, "/repo/two", "Two", None).unwrap();
        assert_ne!(first, second);
    }
}
