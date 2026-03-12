/** OAuth provider identifiers — must match the Rust `Provider` enum. */
export type Provider = "google" | "microsoft";

/** Account metadata stored in SQLite (no secrets). */
export interface Account {
	id: string;
	provider: Provider;
	email: string;
	scopes: string;
	connected_at: string;
}
