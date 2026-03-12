import { beforeEach, describe, expect, it } from "vitest";
import {
	addAccount,
	getAccounts,
	getAccountsByProvider,
	removeAccount,
	setAccounts,
} from "$lib/accounts.svelte";
import type { Account } from "$lib/types";

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

describe("accounts store (actual module)", () => {
	beforeEach(() => {
		// Reset the store before each test
		setAccounts([]);
	});

	it("starts empty", () => {
		expect(getAccounts()).toHaveLength(0);
	});

	it("adds an account to the store", () => {
		addAccount(googleAccount);
		const list = getAccounts();
		expect(list).toHaveLength(1);
		expect(list[0]).toEqual(googleAccount);
	});

	it("prevents duplicate accounts by id", () => {
		addAccount(googleAccount);
		addAccount(googleAccount);
		expect(getAccounts()).toHaveLength(1);
	});

	it("allows multiple accounts of the same provider", () => {
		addAccount(googleAccount);
		addAccount(googleAccount2);
		const list = getAccounts();
		expect(list).toHaveLength(2);
		expect(list[0].provider).toBe("google");
		expect(list[1].provider).toBe("google");
	});

	it("allows accounts from different providers", () => {
		addAccount(googleAccount);
		addAccount(microsoftAccount);
		expect(getAccounts()).toHaveLength(2);
	});

	it("removes an account by id", () => {
		addAccount(googleAccount);
		addAccount(microsoftAccount);
		removeAccount(googleAccount.id);
		const list = getAccounts();
		expect(list).toHaveLength(1);
		expect(list[0].id).toBe(microsoftAccount.id);
	});

	it("removing nonexistent account is a no-op", () => {
		addAccount(googleAccount);
		removeAccount("nonexistent-id");
		expect(getAccounts()).toHaveLength(1);
	});

	it("filters accounts by provider", () => {
		addAccount(googleAccount);
		addAccount(microsoftAccount);
		addAccount(googleAccount2);

		const googleOnly = getAccountsByProvider("google");
		expect(googleOnly).toHaveLength(2);
		expect(googleOnly.every((a) => a.provider === "google")).toBe(true);

		const msOnly = getAccountsByProvider("microsoft");
		expect(msOnly).toHaveLength(1);
		expect(msOnly[0].email).toBe("bob@outlook.com");
	});

	it("setAccounts replaces the entire list", () => {
		addAccount(googleAccount);
		setAccounts([microsoftAccount]);
		const list = getAccounts();
		expect(list).toHaveLength(1);
		expect(list[0].id).toBe(microsoftAccount.id);
	});

	it("disconnect flow: remove from store after disconnect", () => {
		addAccount(googleAccount);
		addAccount(microsoftAccount);
		// Simulate the disconnect flow
		const accountToRemove = googleAccount.id;
		removeAccount(accountToRemove);
		const list = getAccounts();
		expect(list).toHaveLength(1);
		expect(list.find((a) => a.id === accountToRemove)).toBeUndefined();
	});
});
