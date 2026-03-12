import { invoke } from "@tauri-apps/api/core";
import type { Account, Provider } from "./types";

/**
 * Get the OAuth authorisation URL for a provider.
 * The frontend should open this URL in the system browser.
 */
export async function getAuthUrl(provider: Provider): Promise<string> {
	return invoke<string>("get_auth_url", { provider });
}

/**
 * Exchange an OAuth callback (state + code) for an Account.
 * Called after the localhost redirect captures the authorisation code.
 */
export async function handleOAuthCallback(
	state: string,
	code: string,
): Promise<Account> {
	return invoke<Account>("handle_oauth_callback", { state, code });
}

/**
 * Disconnect an account — removes tokens from keychain.
 * The caller should also remove the row from the SQLite accounts table.
 */
export async function disconnectAccount(accountId: string): Promise<void> {
	return invoke<void>("disconnect_account", { accountId });
}

/**
 * Get a valid access token for an account (refreshes if needed).
 */
export async function getValidToken(
	accountId: string,
	provider: Provider,
): Promise<string> {
	return invoke<string>("get_valid_token", { accountId, provider });
}
