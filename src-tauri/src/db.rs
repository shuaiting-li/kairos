use tauri_plugin_sql::{Migration, MigrationKind};

pub fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "create initial schema",
        sql: "CREATE TABLE IF NOT EXISTS _kairos_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        kind: MigrationKind::Up,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_not_empty() {
        assert!(!migrations().is_empty());
    }
}
