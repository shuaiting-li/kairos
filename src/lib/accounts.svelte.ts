import type { Account, Provider } from "./types";

/**
 * Simple reactive accounts store.
 * In a real app this would sync with the SQLite accounts table via
 * tauri-plugin-sql. For now it uses an in-memory array wrapped with
 * Svelte 5 runes ($state).
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
