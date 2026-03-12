/**
 * SQLite persistence layer for accounts via tauri-plugin-sql.
 *
 * The accounts table schema:
 *   id TEXT PRIMARY KEY,
 *   provider TEXT NOT NULL,
 *   email TEXT NOT NULL,
 *   scopes TEXT NOT NULL,
 *   connected_at TEXT NOT NULL
 */

import Database from "@tauri-apps/plugin-sql";
import type { Account } from "./types";

let db: Awaited<ReturnType<typeof Database.load>> | null = null;

async function getDb() {
	if (!db) {
		db = await Database.load("sqlite:kairos.db");
	}
	return db;
}

/** Load all accounts from SQLite. */
export async function loadAccounts(): Promise<Account[]> {
	const conn = await getDb();
	const rows = await conn.select<Account[]>(
		"SELECT id, provider, email, scopes, connected_at FROM accounts",
	);
	return rows;
}

/** Insert or replace an account in SQLite. */
export async function insertAccount(account: Account): Promise<void> {
	const conn = await getDb();
	await conn.execute(
		"INSERT OR REPLACE INTO accounts (id, provider, email, scopes, connected_at) VALUES (?, ?, ?, ?, ?)",
		[
			account.id,
			account.provider,
			account.email,
			account.scopes,
			account.connected_at,
		],
	);
}

/** Delete an account from SQLite by id. */
export async function deleteAccount(accountId: string): Promise<void> {
	const conn = await getDb();
	await conn.execute("DELETE FROM accounts WHERE id = ?", [accountId]);
}
