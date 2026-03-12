import { describe, expect, it } from "vitest";
import type { Account, Provider } from "$lib/types";

describe("types", () => {
	it("Account object has required fields", () => {
		const account: Account = {
			id: "google-test@example.com",
			provider: "google",
			email: "test@example.com",
			scopes: "openid email",
			connected_at: "2025-01-01T00:00:00Z",
		};
		expect(account.id).toBe("google-test@example.com");
		expect(account.provider).toBe("google");
		expect(account.email).toBe("test@example.com");
		expect(account.scopes).toBe("openid email");
		expect(account.connected_at).toBe("2025-01-01T00:00:00Z");
	});

	it("Provider type accepts google and microsoft", () => {
		const google: Provider = "google";
		const microsoft: Provider = "microsoft";
		expect(google).toBe("google");
		expect(microsoft).toBe("microsoft");
	});

	it("Account can be serialized to JSON and back", () => {
		const account: Account = {
			id: "microsoft-user@outlook.com",
			provider: "microsoft",
			email: "user@outlook.com",
			scopes: "Mail.Read Calendars.ReadWrite",
			connected_at: "2025-06-15T12:30:00Z",
		};
		const json = JSON.stringify(account);
		const parsed: Account = JSON.parse(json);
		expect(parsed).toEqual(account);
	});

	it("multiple accounts of same provider have different ids", () => {
		const account1: Account = {
			id: "google-a@example.com",
			provider: "google",
			email: "a@example.com",
			scopes: "openid email",
			connected_at: "2025-01-01T00:00:00Z",
		};
		const account2: Account = {
			id: "google-b@example.com",
			provider: "google",
			email: "b@example.com",
			scopes: "openid email",
			connected_at: "2025-01-02T00:00:00Z",
		};
		expect(account1.id).not.toBe(account2.id);
		expect(account1.provider).toBe(account2.provider);
	});
});
