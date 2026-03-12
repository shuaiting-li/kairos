import type { Account, Provider } from "./types";

/**
 * Reactive accounts store backed by SQLite via tauri-plugin-sql.
 *
 * On startup, loadAccountsFromDb() is called to restore persisted accounts.
 * When an OAuth callback succeeds, the callback_server emits an
 * "account-connected" Tauri event; the frontend listens for it, persists
 * the new account to SQLite, and adds it to this store.
 *
 * On disconnect, the frontend removes the account from both SQLite and
 * this store.
 */

let accounts: Account[] = $state([]);

export function getAccounts(): Account[] {
	return accounts;
}

export function addAccount(account: Account): void {
	// Prevent duplicates (same id)
	if (!accounts.some((a) => a.id === account.id)) {
		accounts = [...accounts, account];
	}
}

export function removeAccount(accountId: string): void {
	accounts = accounts.filter((a) => a.id !== accountId);
}

export function setAccounts(newAccounts: Account[]): void {
	accounts = newAccounts;
}

export function getAccountsByProvider(provider: Provider): Account[] {
	return accounts.filter((a) => a.provider === provider);
}
