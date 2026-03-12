import { describe, expect, it } from "vitest";
import type { Account, Provider } from "$lib/types";

describe("types", () => {
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

	it("Account id follows provider-email convention", () => {
		const account: Account = {
			id: "google-a@example.com",
			provider: "google",
			email: "a@example.com",
			scopes: "openid email",
			connected_at: "2025-01-01T00:00:00Z",
		};
		expect(account.id).toBe(`${account.provider}-${account.email}`);
	});

	it("Provider is a union of google and microsoft", () => {
		const providers: Provider[] = ["google", "microsoft"];
		expect(providers).toHaveLength(2);
		expect(providers).toContain("google");
		expect(providers).toContain("microsoft");
	});
});
