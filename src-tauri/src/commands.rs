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

/// Returns the OAuth authorisation URL the frontend should open in the browser.
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
    let (client_id, client_secret) = {
        // We don't know the provider yet — it's encoded in the state.
        // exchange_code will look it up from the pending-states map.
        // We need both sets of credentials available.
        // exchange_code internally resolves the provider from the state param.
        //
        // For simplicity, we pass all credentials and let exchange_code pick.
        // But exchange_code needs a single pair, so we try to resolve first.
        //
        // Actually, exchange_code pulls the provider from the pending map, so
        // we just need to know *after* the exchange which provider it was.
        // The cleanest approach: do a two-step flow.
        ("", "")
    };
    let _ = (client_id, client_secret);

    // We need to figure out the provider from the pending state, but
    // exchange_code consumes it. Instead, we peek at the provider first.
    // Since we can't peek without consuming, we'll just try with both sets
    // of credentials — exchange_code resolves the provider internally.

    // Try the exchange (it will find the provider from the state map)
    // First, let's get a temporary clone of config values to avoid borrow issues
    let google_id = config.google_client_id.clone();
    let google_secret = config.google_client_secret.clone();
    let ms_id = config.microsoft_client_id.clone();
    let ms_secret = config.microsoft_client_secret.clone();

    // We need to know the provider for the client_id/secret, but the state
    // map has it. Let's use a helper that peeks at the provider.
    let provider = peek_provider(&state)?;
    let (cid, csecret) = match provider {
        Provider::Google => (google_id, google_secret),
        Provider::Microsoft => (ms_id, ms_secret),
    };

    let (provider, token_data) = oauth::exchange_code(&state, &code, &cid, &csecret).await?;

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

/// Helper to peek at the provider for a given state without consuming it.
fn peek_provider(state: &str) -> Result<Provider, OAuthError> {
    // We need to peek into the PENDING_STATES map.
    // Since it's private in oauth.rs, we'll use a different approach:
    // Try to build a dummy exchange and catch. Actually, the cleanest way
    // is to expose a peek function from oauth.rs.
    //
    // For now, we'll match both — this is safe because exchange_code
    // internally resolves the provider.
    //
    // Actually let's just expose a peek from the oauth module.
    oauth::peek_pending_provider(state)
}

/// Disconnect an account: remove tokens from keychain.
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
