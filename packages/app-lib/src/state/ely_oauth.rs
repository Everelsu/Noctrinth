//! Signing in on Ely.by's own page, rather than in a form of ours.
//!
//! The launcher used to ask for an Ely.by password directly and hand it to
//! `authserver.ely.by`. That works, and it still does — see [`super::ely_auth`],
//! which keeps every account signed in that way alive. What it cannot be is
//! convincing: a program asking for somebody else's password in its own window
//! is exactly the shape of the thing players are told never to trust, and being
//! the honest one changes nothing about how it looks.
//!
//! So the password is typed where it belongs. The launcher opens Ely.by's
//! authorization page, Ely.by decides who the player is — with whatever it asks
//! for today and whatever it asks for next year — and hands back a code the
//! launcher trades for a token.
//!
//! ## No secret
//!
//! Ely.by registers this as a *public* client: one that ships to players and
//! therefore cannot keep a secret, because anything compiled into the launcher
//! is readable by anybody holding the launcher. There is no `client_secret` to
//! be had, and the client id below is not one either.
//!
//! What takes its place is PKCE ([RFC 7636]). Before sending the player to
//! Ely.by the launcher invents a random `code_verifier`, keeps it, and sends
//! only its SHA-256 hash. The code that comes back is worthless to anyone who
//! did not invent that verifier — so intercepting the redirect, which is the
//! one attack a desktop app is genuinely exposed to, buys nothing.
//!
//! [RFC 7636]: https://datatracker.ietf.org/doc/html/rfc7636

use base64::Engine;
use rand::Rng;
use rand::distributions::Alphanumeric;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::util::fetch::INSECURE_REQWEST_CLIENT;

/// Noctrinth's application at <https://account.ely.by/dev/applications>.
///
/// Public, in the OAuth sense and in the plain one: it identifies the
/// application and protects nothing.
const CLIENT_ID: &str = "noctrinth";

/// Where the player is sent to sign in.
const AUTHORIZE_URL: &str = "https://account.ely.by/oauth2/v1";

/// Where the code is traded for a token.
const TOKEN_URL: &str = "https://account.ely.by/api/oauth2/v1/token";

/// Who the token belongs to.
const ACCOUNT_INFO_URL: &str = "https://account.ely.by/api/account/v1/info";

/// Where Ely.by sends the player back to.
///
/// Nothing is ever served from here. The window watches where it is being sent
/// and takes the code out of the address before the page is fetched, which is
/// what makes this address a place to put a code rather than a page — but it
/// must still be registered with Ely.by exactly as written, because Ely.by
/// compares it character for character and refuses to begin otherwise. A page
/// is published there anyway, so that a player who somehow arrives at it is met
/// by something that explains itself.
pub const REDIRECT_URI: &str = "https://everelsu.github.io/Noctrinth/oauth/ely";

/// What the launcher asks to be allowed to do.
///
/// `account_info` is who the player is. `minecraft_server_session` is what
/// makes the resulting token usable as Minecraft's own session token, so the
/// game launches and servers accept it without anything else being fetched.
/// `offline_access` adds a refresh token, which — unlike the pair the old
/// password flow juggles — does not expire.
const SCOPES: &str = "account_info minecraft_server_session offline_access";

/// A PKCE verifier and the challenge derived from it.
///
/// The verifier never leaves the launcher until the code is being exchanged;
/// only the challenge is sent to Ely.by. Kept together because sending one
/// without holding the other is the mistake this exists to prevent.
pub struct PkcePair {
    verifier: String,
    challenge: String,
}

impl PkcePair {
    /// 64 characters out of the unreserved set, which is comfortably inside
    /// RFC 7636's 43-to-128 and needs no escaping anywhere it is sent.
    pub fn generate() -> Self {
        let verifier: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()));

        Self {
            verifier,
            challenge,
        }
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }
}

/// A sign-in that has been started and is waiting for the player.
pub struct ElyAuthorizationRequest {
    /// Where to send the player.
    pub url: String,
    pub pkce: PkcePair,
    /// Ties the redirect that comes back to the request that started it, so a
    /// redirect the launcher did not ask for is ignored rather than acted on.
    pub state: String,
}

/// Builds the address the sign-in window opens at.
pub fn begin() -> ElyAuthorizationRequest {
    let pkce = PkcePair::generate();
    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let url = format!(
        "{AUTHORIZE_URL}?client_id={client_id}&redirect_uri={redirect_uri}\
         &response_type=code&scope={scope}&state={state}\
         &code_challenge={challenge}&code_challenge_method=S256&prompt=select_account",
        client_id = urlencoding::encode(CLIENT_ID),
        redirect_uri = urlencoding::encode(REDIRECT_URI),
        scope = urlencoding::encode(SCOPES),
        state = urlencoding::encode(&state),
        challenge = urlencoding::encode(&pkce.challenge),
    );

    ElyAuthorizationRequest { url, pkce, state }
}

/// What Ely.by sent back to [`REDIRECT_URI`].
#[derive(Debug)]
pub enum ElyRedirect {
    Code(String),
    /// The player pressed "decline", or Ely.by refused. Carries the machine
    /// readable reason so a refusal can be told apart from a failure.
    Denied(String),
}

/// Reads a redirect the window was about to follow.
///
/// What identifies it is the `state`, not the address. The address is Ely.by's
/// to choose — whatever is written in the application registration, down to a
/// trailing slash — and a launcher that insisted on recognising it would fail
/// silently the moment the two disagreed: the window would follow the redirect,
/// land on a page, and sit there signed in with nobody listening. The state is
/// 32 random characters this launcher issued moments ago and told nobody else,
/// so a navigation carrying it is this sign-in coming back, wherever it points.
///
/// `None` means this is not it — the window is somewhere else in the sign-in,
/// and should be left alone.
pub fn read_redirect(url: &str, expected_state: &str) -> Option<ElyRedirect> {
    let parsed = url::Url::parse(url).ok()?;

    let mut code = None;
    let mut error = None;
    let mut state = None;
    for (key, value) in parsed.query_pairs() {
        match &*key {
            "code" => code = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    // Carrying no state at all is every other page in the sign-in, and is not
    // worth a word. Carrying the wrong one is worth one: it did not come from
    // the sign-in this launcher started.
    let state = state?;
    if state != expected_state {
        tracing::warn!("Ignoring an Ely.by redirect with an unexpected state");
        return None;
    }

    match (code, error) {
        (Some(code), _) => Some(ElyRedirect::Code(code)),
        (None, Some(error)) => Some(ElyRedirect::Denied(error)),
        (None, None) => None,
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct AccountInfo {
    uuid: String,
    username: String,
}

/// What a finished sign-in yields.
pub struct ElyOauthTokens {
    pub uuid: Uuid,
    pub username: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

/// Trades the code for a token, then asks whose it is.
pub async fn exchange_code(
    code: &str,
    verifier: &str,
) -> crate::Result<ElyOauthTokens> {
    let tokens = post_token(&[
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
    ])
    .await?;

    let info = account_info(&tokens.access_token).await?;
    let uuid = Uuid::parse_str(&info.uuid).map_err(|e| {
        crate::ErrorKind::OtherError(format!("Invalid UUID from Ely.by: {e}"))
    })?;

    Ok(ElyOauthTokens {
        uuid,
        username: info.username,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}

/// Exchanges a refresh token for a fresh access token.
///
/// Ely.by does not rotate the refresh token, so the one held stays good; the
/// `Option` is only here because the response may legitimately omit it.
pub async fn refresh(refresh_token: &str) -> crate::Result<ElyOauthTokens> {
    let tokens = post_token(&[
        ("client_id", CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", SCOPES),
    ])
    .await?;

    let info = account_info(&tokens.access_token).await?;
    let uuid = Uuid::parse_str(&info.uuid).map_err(|e| {
        crate::ErrorKind::OtherError(format!("Invalid UUID from Ely.by: {e}"))
    })?;

    Ok(ElyOauthTokens {
        uuid,
        username: info.username,
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .or_else(|| Some(refresh_token.to_string())),
    })
}

async fn post_token(form: &[(&str, &str)]) -> crate::Result<TokenResponse> {
    let response = INSECURE_REQWEST_CLIENT
        .post(TOKEN_URL)
        .form(form)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Ely.by token request failed: {e}"
            ))
        })?;

    let status = response.status();
    let body = response.text().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read the Ely.by token response: {e}"
        ))
    })?;

    if !status.is_success() {
        // Ely.by answers with OAuth's own error shape, which names the problem
        // far better than the status does.
        let described = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| {
                value["error_description"]
                    .as_str()
                    .or_else(|| value["error"].as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| format!("HTTP {status}"));

        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by refused the sign-in: {described}"
        ))
        .into());
    }

    serde_json::from_str(&body).map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to parse the Ely.by token response: {e}"
        ))
        .into()
    })
}

async fn account_info(access_token: &str) -> crate::Result<AccountInfo> {
    let response = INSECURE_REQWEST_CLIENT
        .get(ACCOUNT_INFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Ely.by account request failed: {e}"
            ))
        })?;

    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by would not say whose account this is: HTTP {}",
            response.status()
        ))
        .into());
    }

    response.json().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to parse the Ely.by account response: {e}"
        ))
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_is_the_hash_of_the_verifier() {
        // RFC 7636 appendix B, the worked example every server checks against.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()));

        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn a_generated_verifier_is_one_a_server_will_accept() {
        let pkce = PkcePair::generate();

        assert!((43..=128).contains(&pkce.verifier.len()));
        assert!(
            pkce.verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c))
        );
        assert_ne!(pkce.verifier, PkcePair::generate().verifier);
        assert!(!pkce.challenge.contains('='));
    }

    #[test]
    fn the_authorization_url_carries_what_ely_by_needs() {
        let request = begin();

        assert!(request.url.starts_with(AUTHORIZE_URL));
        assert!(request.url.contains("response_type=code"));
        assert!(request.url.contains("code_challenge_method=S256"));
        assert!(request.url.contains(&format!("state={}", request.state)));
        // The verifier itself must never be in the address the player visits.
        assert!(!request.url.contains(request.pkce.verifier()));
    }

    #[test]
    fn a_redirect_is_only_read_when_it_is_the_one_expected() {
        let state = "abc123";

        assert!(matches!(
            read_redirect(
                &format!("{REDIRECT_URI}?code=xyz&state={state}"),
                state
            ),
            Some(ElyRedirect::Code(code)) if code == "xyz"
        ));
        assert!(matches!(
            read_redirect(
                &format!("{REDIRECT_URI}?error=access_denied&state={state}"),
                state
            ),
            Some(ElyRedirect::Denied(error)) if error == "access_denied"
        ));

        // The address is not what identifies it: Ely.by's registration decides
        // that, and a redirect that carries the state is this sign-in wherever
        // it points.
        assert!(matches!(
            read_redirect(
                &format!("https://example.invalid/anywhere?code=xyz&state={state}"),
                state
            ),
            Some(ElyRedirect::Code(code)) if code == "xyz"
        ));

        // A page mid-sign-in carries no state, a forged one carries the wrong.
        assert!(
            read_redirect("https://account.ely.by/oauth2/v1?code=xyz", state)
                .is_none()
        );
        assert!(
            read_redirect(
                &format!("{REDIRECT_URI}?code=xyz&state=forged"),
                state
            )
            .is_none()
        );
        assert!(read_redirect(REDIRECT_URI, state).is_none());
    }
}
