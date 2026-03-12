use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Microsoft,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Google => write!(f, "google"),
            Provider::Microsoft => write!(f, "microsoft"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = OAuthError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "google" => Ok(Provider::Google),
            "microsoft" => Ok(Provider::Microsoft),
            _ => Err(OAuthError::InvalidProvider(s.to_string())),
        }
    }
}

/// Stored account metadata (persisted in SQLite — no secrets here).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub provider: Provider,
    pub email: String,
    pub scopes: String,
    pub connected_at: String,
}

/// Token pair stored **only** in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // unix timestamp (seconds)
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("No pending PKCE state for this request")]
    NoPendingState,

    #[error("OAuth callback error: {0}")]
    CallbackError(String),

    #[error("Token not found for account {0}")]
    TokenNotFound(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

// Make OAuthError serialisable for Tauri command returns.
impl Serialize for OAuthError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// ---------------------------------------------------------------------------
// PKCE helpers (RFC 7636)
// ---------------------------------------------------------------------------

/// A PKCE code verifier (43-128 unreserved characters) and its S256 challenge.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

/// Generate a PKCE code verifier + challenge pair.
pub fn generate_pkce() -> PkceChallenge {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&bytes);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
    }
}

// ---------------------------------------------------------------------------
// Provider configuration
// ---------------------------------------------------------------------------

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
pub const GOOGLE_SCOPES: &str = "openid email https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/calendar";

pub const MICROSOFT_AUTH_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
pub const MICROSOFT_TOKEN_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";
pub const MICROSOFT_USERINFO_URL: &str = "https://graph.microsoft.com/v1.0/me";
pub const MICROSOFT_SCOPES: &str =
    "openid email offline_access Mail.Read Calendars.ReadWrite User.Read";

pub const REDIRECT_PORT: u16 = 17823;

pub fn redirect_uri() -> String {
    format!("http://localhost:{REDIRECT_PORT}/callback")
}

// ---------------------------------------------------------------------------
// In-memory PKCE state store (maps state → PkceChallenge)
// ---------------------------------------------------------------------------

static PENDING_STATES: Mutex<Option<HashMap<String, (Provider, PkceChallenge)>>> =
    Mutex::new(None);

fn store_pending(state: &str, provider: Provider, pkce: PkceChallenge) {
    let mut guard = PENDING_STATES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(state.to_string(), (provider, pkce));
}

fn take_pending(state: &str) -> Option<(Provider, PkceChallenge)> {
    let mut guard = PENDING_STATES.lock().unwrap();
    guard.as_mut().and_then(|m| m.remove(state))
}

/// Peek at the provider for a pending state without consuming it.
pub fn peek_pending_provider(state: &str) -> Result<Provider, OAuthError> {
    let guard = PENDING_STATES.lock().unwrap();
    guard
        .as_ref()
        .and_then(|m| m.get(state))
        .map(|(p, _)| *p)
        .ok_or(OAuthError::NoPendingState)
}

// ---------------------------------------------------------------------------
// Build the authorisation URL the user should visit
// ---------------------------------------------------------------------------

pub fn build_auth_url(provider: Provider, client_id: &str) -> (String, String) {
    let pkce = generate_pkce();
    let state = URL_SAFE_NO_PAD.encode(Sha256::digest(
        format!("{}{}", pkce.verifier, chrono::Utc::now().timestamp()).as_bytes(),
    ));

    let (auth_url, scopes) = match provider {
        Provider::Google => (GOOGLE_AUTH_URL, GOOGLE_SCOPES),
        Provider::Microsoft => (MICROSOFT_AUTH_URL, MICROSOFT_SCOPES),
    };

    let mut params = vec![
        ("client_id", client_id.to_string()),
        ("response_type", "code".into()),
        ("redirect_uri", redirect_uri()),
        ("scope", scopes.into()),
        ("state", state.clone()),
        ("code_challenge", pkce.challenge.clone()),
        ("code_challenge_method", "S256".into()),
    ];

    if provider == Provider::Google {
        params.push(("access_type", "offline".into()));
        params.push(("prompt", "consent".into()));
    }

    let url = url::Url::parse_with_params(auth_url, &params)
        .expect("valid URL")
        .to_string();

    store_pending(&state, provider, pkce);

    (url, state)
}

// ---------------------------------------------------------------------------
// Exchange authorisation code for tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

pub async fn exchange_code(
    state: &str,
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(Provider, TokenData), OAuthError> {
    let (provider, pkce) = take_pending(state).ok_or(OAuthError::NoPendingState)?;

    let token_url = match provider {
        Provider::Google => GOOGLE_TOKEN_URL,
        Provider::Microsoft => MICROSOFT_TOKEN_URL,
    };

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code".to_string());
    params.insert("code", code.to_string());
    params.insert("redirect_uri", redirect_uri());
    params.insert("client_id", client_id.to_string());
    params.insert("code_verifier", pkce.verifier.clone());

    // Google requires client_secret even for PKCE public clients in some
    // configurations; Microsoft always requires it.
    if !client_secret.is_empty() {
        params.insert("client_secret", client_secret.to_string());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(token_url)
        .form(&params)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| OAuthError::RefreshFailed(e.to_string()))?;

    let token_resp: TokenResponse = resp.json().await?;

    let now = chrono::Utc::now().timestamp();
    let expires_at = now + token_resp.expires_in.unwrap_or(3600) as i64;

    let token_data = TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
        expires_at,
    };

    Ok((provider, token_data))
}

// ---------------------------------------------------------------------------
// Fetch user email from the provider's userinfo endpoint
// ---------------------------------------------------------------------------

pub async fn fetch_user_email(
    provider: Provider,
    access_token: &str,
) -> Result<String, OAuthError> {
    let url = match provider {
        Provider::Google => GOOGLE_USERINFO_URL,
        Provider::Microsoft => MICROSOFT_USERINFO_URL,
    };

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| OAuthError::CallbackError(e.to_string()))?
        .json()
        .await?;

    // Google returns { email: "..." }, Microsoft returns { mail: "..." } or
    // { userPrincipalName: "..." }
    let email = match provider {
        Provider::Google => resp["email"].as_str().unwrap_or("unknown").to_string(),
        Provider::Microsoft => resp["mail"]
            .as_str()
            .or_else(|| resp["userPrincipalName"].as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    Ok(email)
}

// ---------------------------------------------------------------------------
// Token refresh
// ---------------------------------------------------------------------------

pub async fn refresh_access_token(
    provider: Provider,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<TokenData, OAuthError> {
    let token_url = match provider {
        Provider::Google => GOOGLE_TOKEN_URL,
        Provider::Microsoft => MICROSOFT_TOKEN_URL,
    };

    let mut params = HashMap::new();
    params.insert("grant_type", "refresh_token");
    params.insert("refresh_token", refresh_token);
    params.insert("client_id", client_id);
    if !client_secret.is_empty() {
        params.insert("client_secret", client_secret);
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(token_url)
        .form(&params)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| OAuthError::RefreshFailed(e.to_string()))?;

    let token_resp: TokenResponse = resp.json().await?;

    let now = chrono::Utc::now().timestamp();
    let expires_at = now + token_resp.expires_in.unwrap_or(3600) as i64;

    Ok(TokenData {
        access_token: token_resp.access_token,
        // Some providers don't return a new refresh token on every refresh
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string()),
        expires_at,
    })
}

// ---------------------------------------------------------------------------
// Keyring helpers — tokens never touch SQLite or disk
// ---------------------------------------------------------------------------

const KEYRING_SERVICE: &str = "com.kairos.app";

fn keyring_key(account_id: &str) -> String {
    format!("oauth-token-{account_id}")
}

pub fn store_token(account_id: &str, token: &TokenData) -> Result<(), OAuthError> {
    let json = serde_json::to_string(token)?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_key(account_id))
        .map_err(|e| OAuthError::Keyring(e.to_string()))?;
    entry
        .set_password(&json)
        .map_err(|e| OAuthError::Keyring(e.to_string()))?;
    Ok(())
}

pub fn load_token(account_id: &str) -> Result<TokenData, OAuthError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_key(account_id))
        .map_err(|e| OAuthError::Keyring(e.to_string()))?;
    let json = entry
        .get_password()
        .map_err(|_| OAuthError::TokenNotFound(account_id.to_string()))?;
    let token: TokenData = serde_json::from_str(&json)?;
    Ok(token)
}

pub fn delete_token(account_id: &str) -> Result<(), OAuthError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_key(account_id))
        .map_err(|e| OAuthError::Keyring(e.to_string()))?;
    // Ignore "not found" errors when deleting
    let _ = entry.delete_credential();
    Ok(())
}

// ---------------------------------------------------------------------------
// Ensure a valid access token — refresh if expired
// ---------------------------------------------------------------------------

pub async fn ensure_valid_token(
    account_id: &str,
    provider: Provider,
    client_id: &str,
    client_secret: &str,
) -> Result<String, OAuthError> {
    let token = load_token(account_id)?;

    let now = chrono::Utc::now().timestamp();
    // Refresh if token expires within 60 seconds
    if token.expires_at > now + 60 {
        return Ok(token.access_token);
    }

    if token.refresh_token.is_empty() {
        return Err(OAuthError::RefreshFailed(
            "No refresh token available".into(),
        ));
    }

    let new_token =
        refresh_access_token(provider, &token.refresh_token, client_id, client_secret).await?;

    store_token(account_id, &new_token)?;

    Ok(new_token.access_token)
}

// ---------------------------------------------------------------------------
// Init (called from lib.rs setup)
// ---------------------------------------------------------------------------

pub fn init() {
    let mut guard = PENDING_STATES.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_does_not_panic() {
        init();
    }

    #[test]
    fn pkce_generates_valid_pair() {
        let pkce = generate_pkce();
        // Verifier should be base64url-encoded 32 bytes → 43 chars
        assert!(!pkce.verifier.is_empty());
        assert!(pkce.verifier.len() >= 43);
        // Challenge should be base64url(sha256(verifier)) → 43 chars
        assert!(!pkce.challenge.is_empty());
        assert_eq!(pkce.challenge.len(), 43);
    }

    #[test]
    fn pkce_challenge_matches_verifier() {
        let pkce = generate_pkce();
        let digest = Sha256::digest(pkce.verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn pkce_generates_unique_pairs() {
        let a = generate_pkce();
        let b = generate_pkce();
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn build_google_auth_url_contains_required_params() {
        init();
        let (url, state) = build_auth_url(Provider::Google, "test-client-id");
        assert!(url.contains("accounts.google.com"));
        assert!(url.contains("test-client-id"));
        assert!(url.contains("code_challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains(&state));
    }

    #[test]
    fn build_microsoft_auth_url_contains_required_params() {
        init();
        let (url, state) = build_auth_url(Provider::Microsoft, "test-client-id");
        assert!(url.contains("login.microsoftonline.com"));
        assert!(url.contains("test-client-id"));
        assert!(url.contains("code_challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&state));
    }

    #[test]
    fn provider_roundtrip() {
        assert_eq!("google".parse::<Provider>().unwrap(), Provider::Google);
        assert_eq!(
            "microsoft".parse::<Provider>().unwrap(),
            Provider::Microsoft
        );
        assert!("invalid".parse::<Provider>().is_err());
    }

    #[test]
    fn provider_display() {
        assert_eq!(Provider::Google.to_string(), "google");
        assert_eq!(Provider::Microsoft.to_string(), "microsoft");
    }

    #[test]
    fn pending_state_store_and_take() {
        init();
        let pkce = generate_pkce();
        store_pending("test-state", Provider::Google, pkce.clone());

        let taken = take_pending("test-state");
        assert!(taken.is_some());
        let (provider, taken_pkce) = taken.unwrap();
        assert_eq!(provider, Provider::Google);
        assert_eq!(taken_pkce.verifier, pkce.verifier);

        // Second take should return None
        assert!(take_pending("test-state").is_none());
    }

    #[test]
    fn take_nonexistent_state_returns_none() {
        init();
        assert!(take_pending("nonexistent-state").is_none());
    }

    #[test]
    fn token_data_serialization_roundtrip() {
        let token = TokenData {
            access_token: "access-123".into(),
            refresh_token: "refresh-456".into(),
            expires_at: 1700000000,
        };
        let json = serde_json::to_string(&token).unwrap();
        let parsed: TokenData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "access-123");
        assert_eq!(parsed.refresh_token, "refresh-456");
        assert_eq!(parsed.expires_at, 1700000000);
    }

    #[test]
    fn redirect_uri_format() {
        let uri = redirect_uri();
        assert!(uri.starts_with("http://localhost:"));
        assert!(uri.ends_with("/callback"));
    }

    #[test]
    fn oauth_error_display() {
        let err = OAuthError::InvalidProvider("foobar".into());
        assert!(err.to_string().contains("foobar"));

        let err = OAuthError::TokenNotFound("acct-1".into());
        assert!(err.to_string().contains("acct-1"));
    }

    #[test]
    fn oauth_error_serializes_as_string() {
        let err = OAuthError::InvalidProvider("test".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Invalid provider: test"));
    }

    #[test]
    fn peek_pending_provider_returns_correct_provider() {
        init();
        let pkce = generate_pkce();
        store_pending("peek-test-state", Provider::Microsoft, pkce);

        let provider = peek_pending_provider("peek-test-state").unwrap();
        assert_eq!(provider, Provider::Microsoft);

        // Peek does NOT consume — take should still work
        let taken = take_pending("peek-test-state");
        assert!(taken.is_some());
    }

    #[test]
    fn peek_pending_provider_returns_error_for_unknown_state() {
        init();
        let result = peek_pending_provider("unknown-state");
        assert!(result.is_err());
    }
}
