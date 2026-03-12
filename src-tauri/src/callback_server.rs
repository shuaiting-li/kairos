//! Localhost HTTP server that captures OAuth redirect callbacks.
//!
//! When the user completes OAuth consent in their browser, the provider
//! redirects to `http://localhost:17823/callback?code=...&state=...`.
//! This module spins up a minimal TCP server on that port, extracts the
//! query parameters, performs the token exchange, stores tokens in the
//! OS keychain, and emits a Tauri event so the frontend can persist the
//! account to SQLite and update the UI.

use crate::commands::OAuthConfig;
use crate::oauth::{self, OAuthError, REDIRECT_PORT};
use log::{error, info};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Tauri event name emitted when an OAuth flow completes successfully.
pub const EVENT_ACCOUNT_CONNECTED: &str = "account-connected";
/// Tauri event name emitted when an OAuth flow fails.
pub const EVENT_ACCOUNT_ERROR: &str = "account-error";

/// Start the localhost OAuth callback server in a background tokio task.
/// Must be called from within the Tauri `setup` hook so we have an AppHandle.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tokio::spawn(async move {
        if let Err(e) = run_server(handle).await {
            error!("OAuth callback server error: {}", e);
        }
    });
}

async fn run_server(app: AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("127.0.0.1:{REDIRECT_PORT}");
    let listener = TcpListener::bind(&addr).await?;
    info!("OAuth callback server listening on {}", addr);

    loop {
        let (mut stream, _) = listener.accept().await?;
        let app = app.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let n = match stream.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);

            // Parse the GET request line: "GET /callback?code=...&state=... HTTP/1.1"
            let (code, state) = match parse_callback_params(&request) {
                Some(params) => params,
                None => {
                    let _ = send_response(
                        &mut stream,
                        400,
                        "Invalid callback request. Missing code or state parameter.",
                    )
                    .await;
                    return;
                }
            };

            // Process the OAuth callback
            match process_callback(&app, &state, &code).await {
                Ok(email) => {
                    let _ = send_response(
                        &mut stream,
                        200,
                        &format!(
                            "<!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:60px\">\
                            <h1>&#x2705; Connected!</h1>\
                            <p>Account <strong>{}</strong> has been connected to Kairos.</p>\
                            <p>You can close this tab and return to the app.</p>\
                            </body></html>",
                            html_escape(&email)
                        ),
                    )
                    .await;
                }
                Err(e) => {
                    error!("OAuth callback processing failed: {}", e);
                    let _ = app.emit(EVENT_ACCOUNT_ERROR, e.to_string());
                    let _ = send_response(
                        &mut stream,
                        500,
                        &format!(
                            "<!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:60px\">\
                            <h1>&#x274C; Connection Failed</h1>\
                            <p>{}</p>\
                            <p>Please close this tab and try again in Kairos.</p>\
                            </body></html>",
                            html_escape(&e.to_string())
                        ),
                    )
                    .await;
                }
            }
        });
    }
}

/// Process an OAuth callback: exchange code, fetch email, store token in keychain,
/// emit event for frontend to persist account and update UI.
async fn process_callback(
    app: &AppHandle,
    state: &str,
    code: &str,
) -> Result<String, OAuthError> {
    // Atomically take the pending state (single operation — no peek-then-take race)
    let (provider, pkce) = oauth::take_pending_state(state)?;

    // Get the OAuth config from Tauri managed state
    let config = app.state::<OAuthConfig>();
    let (client_id, client_secret) = match provider {
        oauth::Provider::Google => (
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        ),
        oauth::Provider::Microsoft => (
            config.microsoft_client_id.clone(),
            config.microsoft_client_secret.clone(),
        ),
    };

    // Exchange the code for tokens
    let token_data =
        oauth::exchange_code(provider, code, &client_id, &client_secret, &pkce.verifier).await?;

    // Fetch user email
    let email = oauth::fetch_user_email(provider, &token_data.access_token).await?;

    // Create account ID
    let account_id = format!("{}-{}", provider, email);

    // Store tokens in OS keychain (never in SQLite!)
    oauth::store_token(&account_id, &token_data)?;

    let scopes = match provider {
        oauth::Provider::Google => oauth::GOOGLE_SCOPES,
        oauth::Provider::Microsoft => oauth::MICROSOFT_SCOPES,
    };

    let connected_at = chrono::Utc::now().to_rfc3339();

    let account = oauth::Account {
        id: account_id,
        provider,
        email: email.clone(),
        scopes: scopes.to_string(),
        connected_at,
    };

    // Emit Tauri event so the frontend can persist to SQLite and update UI
    let _ = app.emit(EVENT_ACCOUNT_CONNECTED, &account);

    Ok(email)
}

/// Parse `code` and `state` from the HTTP request's query string.
fn parse_callback_params(request: &str) -> Option<(String, String)> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;

    if !path.starts_with("/callback") {
        return None;
    }

    let query = path.split('?').nth(1)?;
    let params: HashMap<&str, &str> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();

    let code = url_decode(params.get("code")?);
    let state = url_decode(params.get("state")?);

    Some((code, state))
}

/// Minimal percent-decoding for query parameters.
fn url_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                bytes.push(byte);
            } else {
                bytes.push(b'%');
                bytes.extend_from_slice(hex.as_bytes());
            }
        } else if c == '+' {
            bytes.push(b' ');
        } else {
            // ASCII characters from the query string
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| s.to_string())
}

/// Escape HTML entities to prevent XSS in response pages.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

async fn send_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> Result<(), std::io::Error> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_callback() {
        let request =
            "GET /callback?code=abc123&state=xyz789 HTTP/1.1\r\nHost: localhost:17823\r\n\r\n";
        let (code, state) = parse_callback_params(request).unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz789");
    }

    #[test]
    fn parse_callback_with_encoded_chars() {
        let request =
            "GET /callback?code=abc%20123&state=xyz%3D789 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (code, state) = parse_callback_params(request).unwrap();
        assert_eq!(code, "abc 123");
        assert_eq!(state, "xyz=789");
    }

    #[test]
    fn parse_callback_with_plus_as_space() {
        let request =
            "GET /callback?code=abc+123&state=xyz+789 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (code, state) = parse_callback_params(request).unwrap();
        assert_eq!(code, "abc 123");
        assert_eq!(state, "xyz 789");
    }

    #[test]
    fn parse_callback_missing_code() {
        let request = "GET /callback?state=xyz789 HTTP/1.1\r\n\r\n";
        assert!(parse_callback_params(request).is_none());
    }

    #[test]
    fn parse_callback_missing_state() {
        let request = "GET /callback?code=abc123 HTTP/1.1\r\n\r\n";
        assert!(parse_callback_params(request).is_none());
    }

    #[test]
    fn parse_wrong_path() {
        let request = "GET /other?code=abc&state=xyz HTTP/1.1\r\n\r\n";
        assert!(parse_callback_params(request).is_none());
    }

    #[test]
    fn parse_empty_request() {
        assert!(parse_callback_params("").is_none());
    }

    #[test]
    fn html_escape_prevents_xss() {
        let input = "<script>alert('xss')</script>";
        let escaped = html_escape(input);
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
    }

    #[test]
    fn html_escape_handles_ampersand() {
        assert_eq!(html_escape("a&b"), "a&amp;b");
    }

    #[test]
    fn html_escape_handles_quotes() {
        assert_eq!(html_escape("say \"hello\""), "say &quot;hello&quot;");
    }

    #[test]
    fn url_decode_handles_percent_encoding() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a%3Db"), "a=b");
    }

    #[test]
    fn url_decode_passthrough() {
        assert_eq!(url_decode("simple"), "simple");
    }
}
