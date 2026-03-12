use crate::oauth::{self, Account, OAuthError, Provider};
use tauri::State;

// ---------------------------------------------------------------------------
// App-level state holding OAuth client credentials
// ---------------------------------------------------------------------------

pub struct OAuthConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub microsoft_client_id: String,
    pub microsoft_client_secret: String,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Returns the OAuth authorization URL the frontend should open in the browser.
#[tauri::command]
pub fn get_auth_url(
    config: State<'_, OAuthConfig>,
    provider: Provider,
) -> Result<String, OAuthError> {
    let client_id = match provider {
        Provider::Google => &config.google_client_id,
        Provider::Microsoft => &config.microsoft_client_id,
    };
    let (url, _state) = oauth::build_auth_url(provider, client_id);
    Ok(url)
}

/// Called by the localhost callback server when the OAuth provider redirects.
/// Exchanges the code for tokens, fetches the user email, stores tokens in
/// keychain, and returns the new Account.
#[tauri::command]
pub async fn handle_oauth_callback(
    config: State<'_, OAuthConfig>,
    state: String,
    code: String,
) -> Result<Account, OAuthError> {
    // Atomically take pending state — single operation, no race condition
    let (provider, pkce) = oauth::take_pending_state(&state)?;

    let (cid, csecret) = match provider {
        Provider::Google => (
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        ),
        Provider::Microsoft => (
            config.microsoft_client_id.clone(),
            config.microsoft_client_secret.clone(),
        ),
    };

    let token_data =
        oauth::exchange_code(provider, &code, &cid, &csecret, &pkce.verifier).await?;

    // Fetch the user's email address
    let email = oauth::fetch_user_email(provider, &token_data.access_token).await?;

    // Create an account ID
    let account_id = format!("{}-{}", provider, email);

    // Store tokens in the OS keychain (never in SQLite!)
    oauth::store_token(&account_id, &token_data)?;

    let scopes = match provider {
        Provider::Google => oauth::GOOGLE_SCOPES,
        Provider::Microsoft => oauth::MICROSOFT_SCOPES,
    };

    let account = Account {
        id: account_id,
        provider,
        email,
        scopes: scopes.to_string(),
        connected_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(account)
}

/// Disconnect an account: remove tokens from keychain.
/// The frontend is responsible for also removing the row from SQLite
/// and updating the UI store.
#[tauri::command]
pub fn disconnect_account(account_id: String) -> Result<(), OAuthError> {
    oauth::delete_token(&account_id)?;
    Ok(())
}

/// Get a valid access token for an account (refreshes if needed).
#[tauri::command]
pub async fn get_valid_token(
    config: State<'_, OAuthConfig>,
    account_id: String,
    provider: Provider,
) -> Result<String, OAuthError> {
    let (client_id, client_secret) = match provider {
        Provider::Google => (
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        ),
        Provider::Microsoft => (
            config.microsoft_client_id.clone(),
            config.microsoft_client_secret.clone(),
        ),
    };
    oauth::ensure_valid_token(&account_id, provider, &client_id, &client_secret).await
}
