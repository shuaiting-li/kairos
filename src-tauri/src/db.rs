use tauri_plugin_sql::{Migration, MigrationKind};

pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create initial schema",
            sql: "CREATE TABLE IF NOT EXISTS _kairos_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "create accounts table",
            sql: "CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                email TEXT NOT NULL,
                scopes TEXT NOT NULL,
                connected_at TEXT NOT NULL
            );",
            kind: MigrationKind::Up,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_not_empty() {
        assert!(!migrations().is_empty());
    }

    #[test]
    fn migrations_include_accounts_table() {
        let migs = migrations();
        let accounts_mig = migs.iter().find(|m| m.version == 2);
        assert!(accounts_mig.is_some());
        assert!(accounts_mig.unwrap().sql.contains("accounts"));
    }
}
