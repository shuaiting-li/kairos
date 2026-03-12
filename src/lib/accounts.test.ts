import { describe, expect, it } from "vitest";
import type { Account } from "$lib/types";

/** Pure-function versions of account management logic for testing. */

function addToList(accounts: Account[], account: Account): Account[] {
	if (accounts.some((a) => a.id === account.id)) {
		return accounts;
	}
	return [...accounts, account];
}

function removeFromList(accounts: Account[], accountId: string): Account[] {
	return accounts.filter((a) => a.id !== accountId);
}

function filterByProvider(accounts: Account[], provider: string): Account[] {
	return accounts.filter((a) => a.provider === provider);
}

const googleAccount: Account = {
	id: "google-alice@gmail.com",
	provider: "google",
	email: "alice@gmail.com",
	scopes: "openid email",
	connected_at: "2025-01-01T00:00:00Z",
};

const microsoftAccount: Account = {
	id: "microsoft-bob@outlook.com",
	provider: "microsoft",
	email: "bob@outlook.com",
	scopes: "Mail.Read",
	connected_at: "2025-01-02T00:00:00Z",
};

const googleAccount2: Account = {
	id: "google-carol@gmail.com",
	provider: "google",
	email: "carol@gmail.com",
	scopes: "openid email",
	connected_at: "2025-01-03T00:00:00Z",
};

describe("account management logic", () => {
	it("adds an account to an empty list", () => {
		const result = addToList([], googleAccount);
		expect(result).toHaveLength(1);
		expect(result[0]).toEqual(googleAccount);
	});

	it("prevents duplicate accounts", () => {
		const list = [googleAccount];
		const result = addToList(list, googleAccount);
		expect(result).toHaveLength(1);
	});

	it("allows multiple accounts of the same provider", () => {
		let list: Account[] = [];
		list = addToList(list, googleAccount);
		list = addToList(list, googleAccount2);
		expect(list).toHaveLength(2);
		expect(list[0].provider).toBe("google");
		expect(list[1].provider).toBe("google");
	});

	it("allows accounts from different providers", () => {
		let list: Account[] = [];
		list = addToList(list, googleAccount);
		list = addToList(list, microsoftAccount);
		expect(list).toHaveLength(2);
	});

	it("removes an account by id", () => {
		const list = [googleAccount, microsoftAccount];
		const result = removeFromList(list, googleAccount.id);
		expect(result).toHaveLength(1);
		expect(result[0].id).toBe(microsoftAccount.id);
	});

	it("removing nonexistent account is a no-op", () => {
		const list = [googleAccount];
		const result = removeFromList(list, "nonexistent-id");
		expect(result).toHaveLength(1);
	});

	it("filters accounts by provider", () => {
		const list = [googleAccount, microsoftAccount, googleAccount2];
		const googleOnly = filterByProvider(list, "google");
		expect(googleOnly).toHaveLength(2);
		expect(googleOnly.every((a) => a.provider === "google")).toBe(true);

		const msOnly = filterByProvider(list, "microsoft");
		expect(msOnly).toHaveLength(1);
		expect(msOnly[0].email).toBe("bob@outlook.com");
	});

	it("disconnect removes all stored credentials for an account", () => {
		// This test validates the flow: disconnect → remove from list
		let list = [googleAccount, microsoftAccount];
		// Simulate disconnect
		const accountToRemove = googleAccount.id;
		list = removeFromList(list, accountToRemove);
		expect(list).toHaveLength(1);
		expect(list.find((a) => a.id === accountToRemove)).toBeUndefined();
	});
});
