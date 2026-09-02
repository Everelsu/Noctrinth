use crate::ErrorKind;
use crate::util::fetch::INSECURE_REQWEST_CLIENT;
use base64::Engine;
use base64::prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, TimeZone, Utc};
use dashmap::DashMap;
use futures::TryStreamExt;
use heck::ToTitleCase;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use rand::Rng;
use rand::rngs::OsRng;
use reqwest::header::HeaderMap;
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::json;
use sha2::Digest;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::io;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::{Handle, RuntimeFlavor};
use tokio::sync::Mutex;
use tokio::task;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum MinecraftAuthStep {
    GetDeviceToken,
    SisuAuthenticate,
    GetOAuthToken,
    RefreshOAuthToken,
    SisuAuthorize,
    XboxUserAuthorize,
    XstsAuthorize,
    MinecraftToken,
    MinecraftEntitlements,
    MinecraftProfile,
}

#[derive(thiserror::Error, Debug)]
pub enum MinecraftAuthenticationError {
    #[error("Error reading public key during generation")]
    ReadingPublicKey,
    #[error("Failed to serialize private key to PEM: {0}")]
    PEMSerialize(#[from] p256::pkcs8::Error),
    #[error("Failed to serialize body to JSON during step {step:?}: {source}")]
    SerializeBody {
        step: MinecraftAuthStep,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "Failed to deserialize response to JSON during step {step:?}: {source}. Status Code: {status_code} Body: {raw}"
    )]
    DeserializeResponse {
        step: MinecraftAuthStep,
        raw: String,
        #[source]
        source: serde_json::Error,
        status_code: StatusCode,
    },
    #[error("Request failed during step {step:?}: {source}")]
    Request {
        step: MinecraftAuthStep,
        #[source]
        source: reqwest::Error,
    },
    #[error(
        "The Microsoft authentication service is temporarily unavailable, answering with HTTP status {status_code} during step {step:?}. Please try again in a few minutes."
    )]
    ServiceUnavailable {
        step: MinecraftAuthStep,
        status_code: StatusCode,
    },
    #[error("Error reading XBOX Session ID header")]
    NoSessionId,
    #[error("Error reading user hash")]
    NoUserHash,
}

#[derive(Deserialize)]
struct OAuthErrorResponse {
    error: String,
}

impl MinecraftAuthenticationError {
    fn is_invalid_grant(&self) -> bool {
        matches!(
            self,
            Self::DeserializeResponse {
                step: MinecraftAuthStep::RefreshOAuthToken,
                raw,
                status_code: StatusCode::BAD_REQUEST,
                ..
            } if serde_json::from_str::<OAuthErrorResponse>(raw)
                .is_ok_and(|response| response.error == "invalid_grant")
        )
    }

    fn is_service_unavailable(&self) -> bool {
        matches!(self, Self::ServiceUnavailable { .. })
    }
}

/// Whether an error means an authentication service was down, rather than
/// anything being wrong with the account or the request.
fn is_service_unavailable_error(err: &crate::Error) -> bool {
    matches!(
        &*err.raw,
        ErrorKind::MinecraftAuthenticationError(source)
            if source.is_service_unavailable()
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftLoginFlow {
    pub verifier: String,
    pub challenge: String,
    pub session_id: String,
    pub auth_request_uri: String,
}

#[tracing::instrument]
pub async fn login_begin(
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<MinecraftLoginFlow> {
    let verifier = generate_oauth_challenge();
    let result = sha2::Sha256::digest(&verifier);
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(result);

    match sisu_login_begin(&challenge, exec).await {
        Ok((session_id, auth_request_uri)) => Ok(MinecraftLoginFlow {
            verifier,
            challenge,
            session_id,
            auth_request_uri,
        }),
        Err(err) if is_service_unavailable_error(&err) => {
            tracing::warn!(
                "Could not start the Sisu sign-in flow, falling back to the classic one: {err}"
            );

            Ok(MinecraftLoginFlow {
                verifier,
                // An empty session ID marks a flow Sisu knows nothing about
                session_id: String::new(),
                auth_request_uri: classic_auth_request_uri(&challenge),
                challenge,
            })
        }
        Err(err) => Err(err),
    }
}

/// Asks Sisu for the sign-in URL the official launcher uses, which needs a
/// device token bound to a key this device holds.
async fn sisu_login_begin(
    challenge: &str,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<(String, String)> {
    let (pair, current_date) =
        DeviceTokenPair::refresh_and_get_device_token(Utc::now(), exec).await?;

    let (session_id, redirect_uri) = sisu_authenticate(
        &pair.token.token,
        challenge,
        &pair.key,
        current_date,
    )
    .await?;

    Ok((session_id, redirect_uri.value.msa_oauth_redirect))
}

/// The sign-in URL launchers used before Sisu existed. It asks `login.live.com`
/// for the very same authorization code, so the only thing lost by taking it is
/// the Sisu session, which the classic token exchange does not need either.
fn classic_auth_request_uri(challenge: &str) -> String {
    let mut url = Url::parse("https://login.live.com/oauth20_authorize.srf")
        .expect("the classic authorization URL is valid");

    url.query_pairs_mut()
        .append_pair("client_id", MICROSOFT_CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", AUTH_REPLY_URL)
        .append_pair("scope", REQUESTED_SCOPE)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &generate_oauth_challenge())
        .append_pair("prompt", "select_account");

    url.into()
}

#[tracing::instrument]
pub async fn login_finish(
    code: &str,
    flow: MinecraftLoginFlow,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<Credentials> {
    let oauth_token = oauth_token(code, &flow.verifier).await?;

    let xbox_token = xbox_token_for_minecraft(
        Some(&flow.session_id),
        &oauth_token.value.access_token,
        oauth_token.date,
        exec,
    )
    .await?;
    let minecraft_token = minecraft_token(xbox_token).await?;

    minecraft_entitlements(&minecraft_token.access_token).await?;

    let mut credentials = Credentials {
        offline_profile: MinecraftProfile::default(),
        expires: minecraft_token_expiry(&minecraft_token),
        access_token: minecraft_token.access_token,
        refresh_token: oauth_token.value.refresh_token,
        active: true,
    };

    // During login, we need to fetch the online profile at least once to get the
    // player UUID and name to use for the offline profile, in order for that offline
    // profile to make sense. It's also important to modify the returned credentials
    // object, as otherwise continued usage of it will skip the profile cache due to
    // the dummy UUID
    let online_profile = credentials
        .online_profile()
        .await
        .ok_or(io::Error::other("Failed to fetch player profile"))?;
    credentials.offline_profile = MinecraftProfile {
        id: online_profile.id,
        name: online_profile.name.clone(),
        ..credentials.offline_profile
    };

    credentials.upsert(exec).await?;

    Ok(credentials)
}

#[derive(Deserialize, Debug)]
pub struct Credentials {
    /// The offline profile of the user these credentials are for.
    ///
    /// Such a profile can only be relied upon to have a proper player UUID, which is
    /// never changed. A potentially stale username may be available, but no other data
    /// such as skins or capes is available.
    #[serde(rename = "profile")]
    pub offline_profile: MinecraftProfile,
    pub access_token: String,
    pub refresh_token: String,
    pub expires: DateTime<Utc>,
    pub active: bool,
}

/// An entry in the player profile cache, keyed by player UUID.
pub(super) enum ProfileCacheEntry {
    /// A cached profile that is valid, even though it may be stale.
    Hit(Arc<MinecraftProfile>),
    /// A negative profile fetch result due to an authentication error,
    /// from which we're recovering by holding off from repeatedly
    /// attempting to fetch the profile until the token is refreshed
    /// or some time has passed.
    AuthErrorBackoff {
        likely_expired_token: String,
        last_attempt: Instant,
    },
}

/// A thread-safe cache of online profiles, used to avoid fetching the
/// same profile multiple times as long as they don't get too stale.
///
/// The cache has to be static because credential objects are short lived
/// and disposable, and in the future several threads may be interested in
/// profile data.
pub(super) static PROFILE_CACHE: Mutex<
    HashMap<Uuid, ProfileCacheEntry, BuildHasherDefault<DefaultHasher>>,
> = Mutex::const_new(HashMap::with_hasher(BuildHasherDefault::new()));

const ONLINE_PROFILE_CACHE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(60);
const ONLINE_PROFILE_LIVE_STATE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(5);
const ONLINE_PROFILE_AUTH_ERROR_BACKOFF: std::time::Duration =
    std::time::Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
enum OnlineProfileCacheIntent {
    NormalRead,
    LiveStateRead,
    RefreshFromMojang,
}

impl OnlineProfileCacheIntent {
    fn max_age(self) -> std::time::Duration {
        match self {
            Self::NormalRead => ONLINE_PROFILE_CACHE_MAX_AGE,
            Self::LiveStateRead => ONLINE_PROFILE_LIVE_STATE_MAX_AGE,
            Self::RefreshFromMojang => std::time::Duration::ZERO,
        }
    }

    fn can_use_stale_on_fetch_error(self) -> bool {
        matches!(self, Self::LiveStateRead)
    }
}

impl Credentials {
    /// Refreshes the authentication tokens for this user if they are expired, or
    /// very close to expiration.
    async fn refresh(
        &mut self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        // Use a margin of 5 minutes to give e.g. Minecraft and potentially
        // other operations that depend on a fresh token 5 minutes to complete
        // from now, and deal with some classes of clock skew
        if self.expires > Utc::now() + Duration::minutes(5) {
            return Ok(());
        }

        let oauth_token = oauth_refresh(&self.refresh_token).await?;

        let xbox_token = xbox_token_for_minecraft(
            None,
            &oauth_token.value.access_token,
            oauth_token.date,
            exec,
        )
        .await?;
        let minecraft_token = minecraft_token(xbox_token).await?;

        self.expires = minecraft_token_expiry(&minecraft_token);
        self.access_token = minecraft_token.access_token;
        self.refresh_token = oauth_token.value.refresh_token;

        self.upsert(exec).await?;

        Ok(())
    }

    /// Returns online profile data when the cached copy is still recent enough.
    #[tracing::instrument(skip(self))]
    pub async fn online_profile(&self) -> Option<Arc<MinecraftProfile>> {
        self.online_profile_with_cache_intent(
            OnlineProfileCacheIntent::NormalRead,
        )
        .await
    }

    /// Returns profile data recent enough for skin and cape state.
    ///
    /// Reuses a profile read from the last few seconds so opening the skins page
    /// does not send several identical Mojang requests.
    #[tracing::instrument(skip(self))]
    pub async fn online_profile_fresh(&self) -> Option<Arc<MinecraftProfile>> {
        self.online_profile_with_cache_intent(
            OnlineProfileCacheIntent::LiveStateRead,
        )
        .await
    }

    /// Fetches the online profile from Mojang after a skin or cape change.
    #[tracing::instrument(skip(self))]
    pub async fn refresh_online_profile(
        &self,
    ) -> Option<Arc<MinecraftProfile>> {
        self.online_profile_with_cache_intent(
            OnlineProfileCacheIntent::RefreshFromMojang,
        )
        .await
    }

    async fn online_profile_with_cache_intent(
        &self,
        cache_intent: OnlineProfileCacheIntent,
    ) -> Option<Arc<MinecraftProfile>> {
        let max_age = cache_intent.max_age();
        let stale_profile = {
            let mut profile_cache = PROFILE_CACHE.lock().await;
            let mut remove_cached_entry = false;

            let stale_profile = if let Some(cache_entry) =
                profile_cache.get(&self.offline_profile.id)
            {
                match cache_entry {
                    ProfileCacheEntry::Hit(profile)
                        if profile.is_fresh(max_age) =>
                    {
                        return Some(Arc::clone(profile));
                    }
                    ProfileCacheEntry::Hit(profile) => {
                        Some(Arc::clone(profile))
                    }
                    // Auth errors must be handled with a backoff strategy because it
                    // has been experimentally found that Mojang quickly rate limits
                    // the profile data endpoint on repeated attempts with bad auth
                    ProfileCacheEntry::AuthErrorBackoff {
                        likely_expired_token,
                        last_attempt,
                    } if &self.access_token != likely_expired_token
                        || Instant::now()
                            .saturating_duration_since(*last_attempt)
                            > ONLINE_PROFILE_AUTH_ERROR_BACKOFF =>
                    {
                        remove_cached_entry = true;
                        None
                    }
                    ProfileCacheEntry::AuthErrorBackoff { .. } => {
                        return None;
                    }
                }
            } else {
                None
            };

            if remove_cached_entry {
                profile_cache.remove(&self.offline_profile.id);
            }

            stale_profile
        };

        match minecraft_profile(&self.access_token).await {
            Ok(profile) => {
                let profile = Arc::new(profile);
                let cache_entry = ProfileCacheEntry::Hit(Arc::clone(&profile));

                let mut profile_cache = PROFILE_CACHE.lock().await;
                if self.offline_profile.id != profile.id {
                    profile_cache.remove(&self.offline_profile.id);
                }
                profile_cache.insert(profile.id, cache_entry);

                Some(profile)
            }
            Err(
                err @ MinecraftAuthenticationError::DeserializeResponse {
                    status_code: StatusCode::UNAUTHORIZED,
                    ..
                },
            ) => {
                tracing::warn!(
                    "Failed to fetch online profile for UUID {} likely due to stale credentials, backing off: {err}",
                    self.offline_profile.id
                );

                let mut profile_cache = PROFILE_CACHE.lock().await;
                profile_cache.insert(
                    self.offline_profile.id,
                    ProfileCacheEntry::AuthErrorBackoff {
                        likely_expired_token: self.access_token.clone(),
                        last_attempt: Instant::now(),
                    },
                );

                None
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to fetch online profile for UUID {}: {err}",
                    self.offline_profile.id
                );

                if cache_intent.can_use_stale_on_fetch_error() {
                    stale_profile
                } else {
                    None
                }
            }
        }
    }

    /// Attempts to fetch the online profile for this user if possible, and if that fails
    /// falls back to the known offline profile data.
    ///
    /// See also the [`online_profile`](Self::online_profile) method.
    pub async fn maybe_online_profile(
        &self,
    ) -> MaybeOnlineMinecraftProfile<'_> {
        let online_profile = self.online_profile().await;
        online_profile.map_or_else(
            || MaybeOnlineMinecraftProfile::Offline(&self.offline_profile),
            MaybeOnlineMinecraftProfile::Online,
        )
    }

    /// Like [`get_active`](Self::get_active), but enforces credentials to be
    /// successfully refreshed unless the network is unreachable or times out.
    #[tracing::instrument]
    pub async fn get_default_credential(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Option<Credentials>> {
        let credentials = Self::get_active(exec).await?;

        if let Some(mut creds) = credentials {
            let res = creds.refresh(exec).await;

            match res {
                Ok(_) => Ok(Some(creds)),
                Err(err) => {
                    if let ErrorKind::MinecraftAuthenticationError(
                        MinecraftAuthenticationError::Request {
                            ref source,
                            ..
                        },
                    ) = *err.raw
                        && (source.is_connect() || source.is_timeout())
                    {
                        return Ok(Some(creds));
                    }

                    // An outage on Microsoft's side says nothing about whether these
                    // credentials are still good, so hold on to them like we do when
                    // the network itself is unreachable, instead of failing whatever
                    // the user was doing
                    if is_service_unavailable_error(&err) {
                        return Ok(Some(creds));
                    }

                    if matches!(
                        &*err.raw,
                        ErrorKind::MinecraftAuthenticationError(source)
                            if source.is_invalid_grant()
                    ) {
                        Self::remove(creds.offline_profile.id, exec).await?;

                        if let Some((_, mut user)) =
                            Self::get_all(exec).await?.into_iter().next()
                        {
                            user.active = true;
                            user.upsert(exec).await?;
                        }

                        return Ok(None);
                    }

                    Err(err)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Fetches the currently selected credentials from the database, attempting
    /// to refresh them if they are expired.
    pub async fn get_active(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Option<Self>> {
        let res = sqlx::query!(
            "
            SELECT
                uuid, active, username, access_token, refresh_token, expires
            FROM minecraft_users
            WHERE active = TRUE
            "
        )
        .fetch_optional(exec)
        .await?;

        Ok(match res {
            Some(x) => {
                let mut credentials = Self {
                    offline_profile: MinecraftProfile {
                        id: Uuid::parse_str(&x.uuid).unwrap_or_default(),
                        name: x.username,
                        ..MinecraftProfile::default()
                    },
                    access_token: x.access_token,
                    refresh_token: x.refresh_token,
                    expires: Utc
                        .timestamp_opt(x.expires, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                    active: x.active == 1,
                };
                credentials.refresh(exec).await.ok();
                Some(credentials)
            }
            None => None,
        })
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<DashMap<Uuid, Self>> {
        let res = sqlx::query!(
            "
            SELECT
                uuid, active, username, access_token, refresh_token, expires
            FROM minecraft_users
            "
        )
        .fetch(exec)
        .try_fold(DashMap::new(), |acc, x| {
            let uuid = Uuid::parse_str(&x.uuid).unwrap_or_default();
            let mut credentials = Self {
                offline_profile: MinecraftProfile {
                    id: uuid,
                    name: x.username,
                    ..MinecraftProfile::default()
                },
                access_token: x.access_token,
                refresh_token: x.refresh_token,
                expires: Utc
                    .timestamp_opt(x.expires, 0)
                    .single()
                    .unwrap_or_else(Utc::now),
                active: x.active == 1,
            };

            async move {
                credentials.refresh(exec).await.ok();
                acc.insert(uuid, credentials);

                Ok(acc)
            }
        })
        .await?;

        Ok(res)
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        let profile = self.maybe_online_profile().await;
        let expires = self.expires.timestamp();
        let uuid = profile.id.as_hyphenated().to_string();

        if self.active {
            sqlx::query!(
                "
                UPDATE minecraft_users
                SET active = FALSE
                ",
            )
            .execute(exec)
            .await?;
        }

        sqlx::query!(
            "
            INSERT INTO minecraft_users (uuid, active, username, access_token, refresh_token, expires)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (uuid) DO UPDATE SET
                active = $2,
                username = $3,
                access_token = $4,
                refresh_token = $5,
                expires = $6
            ",
            uuid,
            self.active,
            profile.name,
            self.access_token,
            self.refresh_token,
            expires,
        )
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn remove(
        uuid: Uuid,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let uuid = uuid.as_hyphenated().to_string();

        sqlx::query!(
            "
            DELETE FROM minecraft_users WHERE uuid = $1
            ",
            uuid,
        )
        .execute(exec)
        .await?;

        Ok(())
    }
}

impl Serialize for Credentials {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Opportunistically hydrate the profile with its online data if possible for frontend
        // consumption, transparently handling all the possible Tokio runtime states the current
        // thread may be in the most efficient way
        let profile = match Handle::try_current().ok() {
            Some(runtime)
                if runtime.runtime_flavor() == RuntimeFlavor::CurrentThread =>
            {
                runtime.block_on(self.maybe_online_profile())
            }
            Some(runtime) => task::block_in_place(|| {
                runtime.block_on(self.maybe_online_profile())
            }),
            None => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_or_else(
                    |_| {
                        MaybeOnlineMinecraftProfile::Offline(
                            &self.offline_profile,
                        )
                    },
                    |runtime| runtime.block_on(self.maybe_online_profile()),
                ),
        };

        let mut ser = serializer.serialize_struct("Credentials", 5)?;
        ser.serialize_field("profile", &*profile)?;
        ser.serialize_field("access_token", &self.access_token)?;
        ser.serialize_field("refresh_token", &self.refresh_token)?;
        ser.serialize_field("expires", &self.expires)?;
        ser.serialize_field("active", &self.active)?;
        ser.end()
    }
}

pub struct DeviceTokenPair {
    pub token: DeviceToken,
    pub key: DeviceTokenKey,
}

impl DeviceTokenPair {
    #[tracing::instrument(skip(exec))]
    async fn refresh_and_get_device_token(
        current_date: DateTime<Utc>,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<(Self, DateTime<Utc>)> {
        let pair = Self::get(exec).await?;

        if let Some(mut pair) = pair {
            if pair.token.not_after > current_date {
                Ok((pair, current_date))
            } else {
                let res = device_token(&pair.key, current_date).await?;

                pair.token = res.value;
                pair.upsert(exec).await?;

                Ok((pair, res.date))
            }
        } else {
            let key = generate_key()?;
            let res = device_token(&key, current_date).await?;

            let pair = Self {
                key,
                token: res.value,
            };

            pair.upsert(exec).await?;

            Ok((pair, res.date))
        }
    }

    async fn get(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
        let res = sqlx::query!(
            r#"
            SELECT
                uuid, private_key, x, y, issue_instant, not_after, token, json(display_claims) as "display_claims!: serde_json::Value"
            FROM minecraft_device_tokens
            "#
        )
            .fetch_optional(exec)
            .await?;

        if let Some(x) = res
            && let Ok(uuid) = Uuid::parse_str(&x.uuid)
            && let Ok(private_key) = SigningKey::from_pkcs8_pem(&x.private_key)
        {
            return Ok(Some(Self {
                token: DeviceToken {
                    issue_instant: Utc
                        .timestamp_opt(x.issue_instant, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                    not_after: Utc
                        .timestamp_opt(x.not_after, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                    token: x.token,
                    display_claims: serde_json::from_value(x.display_claims)
                        .unwrap_or_default(),
                },
                key: DeviceTokenKey {
                    id: uuid,
                    key: private_key,
                    x: x.x,
                    y: x.y,
                },
            }));
        }

        Ok(None)
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let uuid = self.key.id.as_hyphenated().to_string();
        let issue_instant = self.token.issue_instant.timestamp();
        let not_after = self.token.not_after.timestamp();
        let key = self
            .key
            .key
            .to_pkcs8_pem(LineEnding::default())
            .map_err(MinecraftAuthenticationError::PEMSerialize)?
            .to_string();
        let display_claims = serde_json::to_string(&self.token.display_claims)?;

        sqlx::query!(
            "
            INSERT INTO minecraft_device_tokens (id, uuid, private_key, x, y, issue_instant, not_after, token, display_claims)
            VALUES (0, $1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                uuid = $1,
                private_key = $2,
                x = $3,
                y = $4,
                issue_instant = $5,
                not_after = $6,
                token = $7,
                display_claims = jsonb($8)
            ",
            uuid,
            key,
            self.key.x,
            self.key.y,
            issue_instant,
            not_after,
            self.token.token,
            display_claims,
        )
            .execute(exec)
            .await?;

        Ok(())
    }
}

// The official Mojang Minecraft Launcher client ID. Pre-whitelisted by Mojang
// for Xbox Live + Minecraft Services API access, so the Sisu flow below works
// without needing a custom Azure AD registration or aka.ms/AppRegInfo approval.
// This is the same ID used by all major third-party launchers (AstralRinth,
// PrismLauncher, MultiMC, ATLauncher, …).
const MICROSOFT_CLIENT_ID: &str = "00000000402b5328";
const AUTH_REPLY_URL: &str = "https://login.live.com/oauth20_desktop.srf";
const REQUESTED_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
pub const MINECRAFT_SERVICES_USER_AGENT: &str =
    "Modrinth App (support@modrinth.com; https://modrinth.com/app)";

pub struct RequestWithDate<T> {
    pub date: DateTime<Utc>,
    pub value: T,
}

// flow steps
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceToken {
    pub issue_instant: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub token: String,
    pub display_claims: HashMap<String, serde_json::Value>,
}

#[tracing::instrument(skip(key))]
pub async fn device_token(
    key: &DeviceTokenKey,
    current_date: DateTime<Utc>,
) -> Result<RequestWithDate<DeviceToken>, MinecraftAuthenticationError> {
    let res = send_signed_request(
        None,
        "https://device.auth.xboxlive.com/device/authenticate",
        "/device/authenticate",
        json!({
            "Properties": {
                "AuthMethod": "ProofOfPossession",
                "Id": format!("{{{}}}", key.id.to_string().to_uppercase()),
                "DeviceType": "Win32",
                "Version": "10.16.0",
                "ProofKey": {
                    "kty": "EC",
                    "x": key.x,
                    "y": key.y,
                    "crv": "P-256",
                    "alg": "ES256",
                    "use": "sig"
                }
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"

        }),
        key,
        MinecraftAuthStep::GetDeviceToken,
        current_date,
    )
    .await?;

    Ok(RequestWithDate {
        date: res.current_date,
        value: res.body,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RedirectUri {
    pub msa_oauth_redirect: String,
}

#[tracing::instrument(skip(key))]
async fn sisu_authenticate(
    token: &str,
    challenge: &str,
    key: &DeviceTokenKey,
    current_date: DateTime<Utc>,
) -> Result<(String, RequestWithDate<RedirectUri>), MinecraftAuthenticationError>
{
    let res = send_signed_request::<RedirectUri>(
        None,
        "https://sisu.xboxlive.com/authenticate",
        "/authenticate",
        json!({
          "AppId": MICROSOFT_CLIENT_ID,
          "DeviceToken": token,
          "Offers": [
            REQUESTED_SCOPE
          ],
          "Query": {
            "code_challenge": challenge,
            "code_challenge_method": "S256",
            "state": generate_oauth_challenge(),
            "prompt": "select_account"
          },
          "RedirectUri": AUTH_REPLY_URL,
          "Sandbox": "RETAIL",
          "TokenType": "code",
          "TitleId": "1794566092",
        }),
        key,
        MinecraftAuthStep::SisuAuthenticate,
        current_date,
    )
    .await?;

    let session_id = res
        .headers
        .get("X-SessionId")
        .and_then(|x| x.to_str().ok())
        .ok_or_else(|| MinecraftAuthenticationError::NoSessionId)?
        .to_string();

    Ok((
        session_id,
        RequestWithDate {
            date: res.current_date,
            value: res.body,
        },
    ))
}

#[derive(Deserialize)]
struct OAuthToken {
    // pub token_type: String,
    // The Minecraft token obtained with this one lives far longer, and it is
    // the one credentials expire with, so this is of no use to us
    // pub expires_in: u64,
    // pub scope: String,
    pub access_token: String,
    pub refresh_token: String,
    // pub user_id: String,
    // pub foci: String,
}

#[tracing::instrument]
async fn oauth_token(
    code: &str,
    verifier: &str,
) -> Result<RequestWithDate<OAuthToken>, MinecraftAuthenticationError> {
    let mut query = HashMap::new();
    query.insert("client_id", MICROSOFT_CLIENT_ID);
    query.insert("code", code);
    query.insert("code_verifier", verifier);
    query.insert("grant_type", "authorization_code");
    query.insert("redirect_uri", AUTH_REPLY_URL);
    query.insert("scope", REQUESTED_SCOPE);

    let res = auth_retry(|| {
        INSECURE_REQWEST_CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .header("Accept", "application/json")
            .form(&query)
            .send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request {
        source,
        step: MinecraftAuthStep::GetOAuthToken,
    })?;

    let status = res.status();
    let current_date = get_date_header(res.headers());
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::GetOAuthToken,
        }
    })?;

    let body =
        parse_auth_response(text, status, MinecraftAuthStep::GetOAuthToken)?;

    Ok(RequestWithDate {
        date: current_date,
        value: body,
    })
}

#[tracing::instrument]
async fn oauth_refresh(
    refresh_token: &str,
) -> Result<RequestWithDate<OAuthToken>, MinecraftAuthenticationError> {
    let mut query = HashMap::new();
    query.insert("client_id", MICROSOFT_CLIENT_ID);
    query.insert("refresh_token", refresh_token);
    query.insert("grant_type", "refresh_token");
    query.insert("redirect_uri", AUTH_REPLY_URL);
    query.insert("scope", REQUESTED_SCOPE);

    let res = auth_retry(|| {
        INSECURE_REQWEST_CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .header("Accept", "application/json")
            .form(&query)
            .send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request {
        source,
        step: MinecraftAuthStep::RefreshOAuthToken,
    })?;

    let status = res.status();
    let current_date = get_date_header(res.headers());
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::RefreshOAuthToken,
        }
    })?;

    let body = parse_auth_response(
        text,
        status,
        MinecraftAuthStep::RefreshOAuthToken,
    )?;

    Ok(RequestWithDate {
        date: current_date,
        value: body,
    })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SisuAuthorize {
    // pub authorization_token: DeviceToken,
    // pub device_token: String,
    // pub sandbox: String,
    pub title_token: DeviceToken,
    pub user_token: DeviceToken,
    // pub web_page: String,
}

#[tracing::instrument(skip(key))]
async fn sisu_authorize(
    session_id: Option<&str>,
    access_token: &str,
    device_token: &str,
    key: &DeviceTokenKey,
    current_date: DateTime<Utc>,
) -> Result<RequestWithDate<SisuAuthorize>, MinecraftAuthenticationError> {
    let res = send_signed_request(
        None,
        "https://sisu.xboxlive.com/authorize",
        "/authorize",
        json!({
            "AccessToken": format!("t={access_token}"),
            "AppId": MICROSOFT_CLIENT_ID,
            "DeviceToken": device_token,
            "ProofKey": {
                "kty": "EC",
                "x": key.x,
                "y": key.y,
                "crv": "P-256",
                "alg": "ES256",
                "use": "sig"
            },
            "Sandbox": "RETAIL",
            "SessionId": session_id,
            "SiteName": "user.auth.xboxlive.com",
            "RelyingParty": "http://xboxlive.com",
            "UseModernGamertag": true
        }),
        key,
        MinecraftAuthStep::SisuAuthorize,
        current_date,
    )
    .await?;

    Ok(RequestWithDate {
        date: res.current_date,
        value: res.body,
    })
}

/// Trades a Microsoft access token for the Xbox Live token Minecraft services
/// accept.
///
/// The Sisu flow the official launcher uses is preferred, but it lives behind
/// `sisu.xboxlive.com`, which goes down on its own often enough to be worth
/// routing around. The classic flow that launchers used before Sisu existed
/// reaches the same place through entirely different hosts, needing neither a
/// device token nor request signing, so an outage of one is rarely an outage of
/// both.
#[tracing::instrument(skip(access_token, exec))]
async fn xbox_token_for_minecraft(
    session_id: Option<&str>,
    access_token: &str,
    current_date: DateTime<Utc>,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<DeviceToken> {
    // A sign-in that already had to begin without Sisu has no session for it to
    // authorize, so there is nothing to try there
    if !session_id.is_some_and(str::is_empty) {
        match sisu_xbox_token(session_id, access_token, current_date, exec)
            .await
        {
            Ok(token) => return Ok(token),
            Err(err) if is_service_unavailable_error(&err) => {
                tracing::warn!(
                    "Sisu is unavailable, falling back to the classic Xbox Live sign-in: {err}"
                );
            }
            Err(err) => return Err(err),
        }
    }

    Ok(classic_xbox_token(access_token).await?)
}

async fn sisu_xbox_token(
    session_id: Option<&str>,
    access_token: &str,
    current_date: DateTime<Utc>,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<DeviceToken> {
    let (pair, current_date) =
        DeviceTokenPair::refresh_and_get_device_token(current_date, exec)
            .await?;

    let sisu_authorize = sisu_authorize(
        session_id,
        access_token,
        &pair.token.token,
        &pair.key,
        current_date,
    )
    .await?;

    let xbox_token = xsts_authorize(
        sisu_authorize.value,
        &pair.token.token,
        &pair.key,
        sisu_authorize.date,
    )
    .await?;

    Ok(xbox_token.value)
}

async fn classic_xbox_token(
    access_token: &str,
) -> Result<DeviceToken, MinecraftAuthenticationError> {
    let user_token = xbox_user_token(access_token).await?;

    classic_xsts_authorize(&user_token.token).await
}

/// An unsigned Xbox Live token request, as used by the classic flow.
async fn xbox_live_request(
    url: &str,
    body: serde_json::Value,
    step: MinecraftAuthStep,
) -> Result<DeviceToken, MinecraftAuthenticationError> {
    let body = serde_json::to_vec(&body).map_err(|source| {
        MinecraftAuthenticationError::SerializeBody { source, step }
    })?;

    let res = auth_retry(|| {
        INSECURE_REQWEST_CLIENT
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Accept", "application/json")
            .header("x-xbl-contract-version", "1")
            .body(body.clone())
            .send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request { source, step })?;

    let status = res.status();
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request { source, step }
    })?;

    parse_auth_response(text, status, step)
}

/// Exchanges a Microsoft access token for an Xbox Live user token without
/// involving Sisu.
async fn xbox_user_token(
    access_token: &str,
) -> Result<DeviceToken, MinecraftAuthenticationError> {
    xbox_live_request(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                // The ticket format this client ID uses, the same one the Sisu
                // authorize step sends
                "RpsTicket": format!("t={access_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
        MinecraftAuthStep::XboxUserAuthorize,
    )
    .await
}

/// Authorizes an Xbox Live user token for Minecraft services. Unlike the Sisu
/// path, this presents no device or title token, so it is not tied to a device
/// key this launcher registered earlier.
async fn classic_xsts_authorize(
    user_token: &str,
) -> Result<DeviceToken, MinecraftAuthenticationError> {
    xbox_live_request(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [user_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }),
        MinecraftAuthStep::XstsAuthorize,
    )
    .await
}

#[tracing::instrument(skip(key))]
async fn xsts_authorize(
    authorize: SisuAuthorize,
    device_token: &str,
    key: &DeviceTokenKey,
    current_date: DateTime<Utc>,
) -> Result<RequestWithDate<DeviceToken>, MinecraftAuthenticationError> {
    let res = send_signed_request(
        None,
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        "/xsts/authorize",
        json!({
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [authorize.user_token.token],
                "DeviceToken": device_token,
                "TitleToken": authorize.title_token.token,
            },
        }),
        key,
        MinecraftAuthStep::XstsAuthorize,
        current_date,
    )
    .await?;

    Ok(RequestWithDate {
        date: res.current_date,
        value: res.body,
    })
}

#[derive(Deserialize)]
struct MinecraftToken {
    // pub username: String,
    pub access_token: String,
    // pub token_type: String,
    pub expires_in: u64,
}

/// When the credentials built around a Minecraft token should be refreshed.
///
/// The Minecraft token is the only one the game and Minecraft services ever
/// see, and it outlives by a day the Microsoft access token it was obtained
/// with. Expiring credentials on the Microsoft token's hourly schedule would
/// mean walking the whole Xbox Live chain every hour for nothing, and being
/// unable to play whenever Xbox Live happens to be down at that moment.
fn minecraft_token_expiry(token: &MinecraftToken) -> DateTime<Utc> {
    const MAX_TOKEN_LIFETIME: i64 = 24 * 60 * 60;

    // Local time is what this is compared against later, so counting from now
    // rather than from a server date keeps clock skew out of it
    Utc::now()
        + Duration::seconds((token.expires_in as i64).min(MAX_TOKEN_LIFETIME))
}

#[tracing::instrument]
async fn minecraft_token(
    token: DeviceToken,
) -> Result<MinecraftToken, MinecraftAuthenticationError> {
    let uhs = token
        .display_claims
        .get("xui")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("uhs"))
        .and_then(|x| x.as_str().map(String::from))
        .ok_or_else(|| MinecraftAuthenticationError::NoUserHash)?;

    let token = token.token;

    let res = auth_retry(|| {
        INSECURE_REQWEST_CLIENT
            .post("https://api.minecraftservices.com/launcher/login")
            .header("Accept", "application/json")
            .header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
            .json(&json!({
                "platform": "PC_LAUNCHER",
                "xtoken": format!("XBL3.0 x={uhs};{token}"),
            }))
            .send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request {
        source,
        step: MinecraftAuthStep::MinecraftToken,
    })?;

    let status = res.status();
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::MinecraftToken,
        }
    })?;

    parse_auth_response(text, status, MinecraftAuthStep::MinecraftToken)
}

#[derive(
    sqlx::Type, Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq,
)]
#[serde(rename_all = "UPPERCASE")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum MinecraftSkinVariant {
    /// The classic player model, with arms that are 4 pixels wide.
    Classic,
    /// The slim player model, with arms that are 3 pixels wide.
    Slim,
    /// The player model is unknown.
    #[serde(other)]
    Unknown, // Defensive handling of unexpected Mojang API return values to
             // prevent breaking the entire profile parsing
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MinecraftCharacterExpressionState {
    /// This expression is selected for being displayed ingame.
    ///
    /// At the moment, at most one expression can be selected at a time.
    Active,
    /// This expression is not selected for being displayed ingame.
    Inactive,
    /// The expression selection status is unknown.
    #[serde(other)]
    Unknown, // Defensive handling of unexpected Mojang API return values to
             // prevent breaking the entire profile parsing
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MinecraftSkin {
    /// The UUID of this skin object.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint this UUID
    /// changes every time the player changes their skin, even if the skin
    /// texture is the same as before.
    pub id: Uuid,
    /// The selection state of the skin.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint this
    /// is always `ACTIVE`, as only a single skin representing the current
    /// skin is returned.
    pub state: MinecraftCharacterExpressionState,
    /// The URL to the skin texture.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint the file
    /// name for this URL is a hash of the skin texture, so that different
    /// players using the same skin texture will share a texture URL.
    pub url: Arc<Url>,
    /// A hash of the skin texture.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint this
    /// is always set and the same as the file name of the skin texture URL.
    #[serde(
        default, // Defensive handling of unexpected Mojang API return values to
                 // prevent breaking the entire profile parsing
        rename = "textureKey"
    )]
    pub texture_key: Option<Arc<str>>,
    /// The player model variant this skin is for.
    pub variant: MinecraftSkinVariant,
    /// User-friendly name for the skin.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint this is
    /// only set if the player has not set a custom skin, and this skin object
    /// is therefore the default skin for the player's UUID.
    #[serde(
        default,
        rename = "alias",
        deserialize_with = "normalize_skin_alias_case"
    )]
    pub name: Option<String>,
}

impl MinecraftSkin {
    /// Robustly computes the texture key for this skin, falling back to its
    /// URL file name and finally to the skin UUID when necessary.
    pub fn texture_key(&self) -> Arc<str> {
        self.texture_key.as_ref().cloned().unwrap_or_else(|| {
            self.url
                .path_segments()
                .and_then(|mut path_segments| {
                    path_segments.next_back().map(String::from)
                })
                .unwrap_or_else(|| self.id.as_simple().to_string())
                .into()
        })
    }
}

fn normalize_skin_alias_case<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    // Skin aliases have been spotted to be returned in all caps, so make sure
    // they are normalized to a prettier title case
    Ok(<Option<Cow<'_, str>>>::deserialize(deserializer)?
        .map(|alias| alias.to_title_case()))
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MinecraftCape {
    /// The UUID of the cape.
    pub id: Uuid,
    /// The selection state of the cape.
    pub state: MinecraftCharacterExpressionState,
    /// The URL to the cape texture.
    pub url: Arc<Url>,
    /// The user-friendly name for the cape.
    #[serde(rename = "alias")]
    pub name: Arc<str>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct MinecraftProfile {
    /// The UUID of the player.
    #[serde(default)]
    pub id: Uuid,
    /// The username of the player.
    pub name: String,
    /// The skins the player is known to have.
    ///
    /// As of 2025-04-08, in the production Mojang profile endpoint every
    /// player has a single skin.
    pub skins: Vec<MinecraftSkin>,
    /// The capes the player is known to have.
    pub capes: Vec<MinecraftCape>,
    /// The instant when the profile was fetched. See also [Self::is_fresh].
    #[serde(skip)]
    pub fetch_time: Option<Instant>,
}

impl MinecraftProfile {
    /// Checks whether the profile data is fresh (i.e., highly likely to be
    /// up-to-date because it was fetched recently) or stale. If it is not
    /// known when this profile data has been fetched from Mojang servers (i.e.,
    /// `fetch_time` is `None`), the profile is considered stale.
    ///
    /// This can be used to determine if the profile data should be fetched again
    /// from the Mojang API: the vanilla launcher was seen refreshing profile
    /// data every 60 seconds when re-entering the skin selection screen, and
    /// external applications may change this data at any time.
    fn is_fresh(&self, max_age: std::time::Duration) -> bool {
        self.fetch_time.is_some_and(|last_profile_fetch_time| {
            Instant::now().saturating_duration_since(last_profile_fetch_time)
                < max_age
        })
    }

    /// Returns the currently selected skin for this profile.
    pub fn current_skin(&self) -> crate::Result<&MinecraftSkin> {
        Ok(self
            .skins
            .iter()
            .find(|skin| {
                skin.state == MinecraftCharacterExpressionState::Active
            })
            // There should always be one active skin, even when the player uses their default skin
            .ok_or_else(|| {
                ErrorKind::OtherError("No active skin found".into())
            })?)
    }

    /// Returns the currently selected cape for this profile.
    pub fn current_cape(&self) -> Option<&MinecraftCape> {
        self.capes.iter().find(|cape| {
            cape.state == MinecraftCharacterExpressionState::Active
        })
    }
}

pub enum MaybeOnlineMinecraftProfile<'profile> {
    /// An online profile, fetched from the Mojang API.
    Online(Arc<MinecraftProfile>),
    /// An offline profile, which has not been fetched from the Mojang API.
    Offline(&'profile MinecraftProfile),
}

impl Deref for MaybeOnlineMinecraftProfile<'_> {
    type Target = MinecraftProfile;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Online(profile) => profile,
            Self::Offline(profile) => profile,
        }
    }
}

#[tracing::instrument(skip(token))]
async fn minecraft_profile(
    token: &str,
) -> Result<MinecraftProfile, MinecraftAuthenticationError> {
    let res = auth_retry(|| {
        INSECURE_REQWEST_CLIENT
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Accept", "application/json")
            .header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
            .bearer_auth(token)
            // Profiles may be refreshed periodically in response to user actions,
            // so we want each refresh to be fast
            .timeout(std::time::Duration::from_secs(10))
            .send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request {
        source,
        step: MinecraftAuthStep::MinecraftProfile,
    })?;

    let status = res.status();
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::MinecraftProfile,
        }
    })?;

    let mut profile = parse_auth_response::<MinecraftProfile>(
        text,
        status,
        MinecraftAuthStep::MinecraftProfile,
    )?;
    profile.fetch_time = Some(Instant::now());

    tracing::debug!(
        "Successfully fetched Minecraft profile for {}",
        profile.name
    );

    Ok(profile)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftEntitlements {}

#[tracing::instrument]
async fn minecraft_entitlements(
    token: &str,
) -> Result<MinecraftEntitlements, MinecraftAuthenticationError> {
    let res = auth_retry(|| {
		INSECURE_REQWEST_CLIENT
			.get(format!("https://api.minecraftservices.com/entitlements/license?requestId={}", Uuid::new_v4()))
			.header("Accept", "application/json")
			.header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
			.bearer_auth(token)
			.send()
	})
    .await.map_err(|source| MinecraftAuthenticationError::Request { source, step: MinecraftAuthStep::MinecraftEntitlements })?;

    let status = res.status();
    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::MinecraftEntitlements,
        }
    })?;

    parse_auth_response(text, status, MinecraftAuthStep::MinecraftEntitlements)
}

// auth utils

const RETRY_COUNT: usize = 5;
const BASE_RETRY_WAIT: std::time::Duration =
    std::time::Duration::from_millis(250);
const MAX_RETRY_WAIT: std::time::Duration = std::time::Duration::from_secs(4);

/// Whether a status means the authentication service is having a moment, rather
/// than something being wrong with the request itself. Microsoft puts a CDN in
/// front of these endpoints which answers with an HTML error page while the
/// service behind it is down, so such responses are not even JSON.
fn is_transient_status(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
        )
}

/// Parses the JSON body of an authentication response, reporting a body that
/// could not be parsed because the service was failing as the outage it is,
/// instead of as a deserialization error quoting an HTML error page.
fn parse_auth_response<T: DeserializeOwned>(
    text: String,
    status_code: StatusCode,
    step: MinecraftAuthStep,
) -> Result<T, MinecraftAuthenticationError> {
    serde_json::from_str(&text).map_err(|source| {
        if is_transient_status(status_code) {
            tracing::warn!(
                "Authentication step {step:?} failed with status {status_code}: {text}"
            );

            MinecraftAuthenticationError::ServiceUnavailable {
                step,
                status_code,
            }
        } else {
            MinecraftAuthenticationError::DeserializeResponse {
                source,
                raw: text,
                step,
                status_code,
            }
        }
    })
}

/// The delay the service asked us to hold off for, when it sent one.
fn retry_after(response: &Response) -> Option<std::time::Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(std::time::Duration::from_secs)
}

/// A wait that grows exponentially with each attempt, with some jitter so that
/// every client retrying at once does not come back in the same instant.
fn backoff_wait(attempt: usize) -> std::time::Duration {
    let wait = BASE_RETRY_WAIT
        .saturating_mul(1u32 << (attempt.min(4) as u32))
        .min(MAX_RETRY_WAIT);

    wait + wait.mul_f32(rand::thread_rng().gen_range(0.0..0.25))
}

#[tracing::instrument(skip(reqwest_request))]
async fn auth_retry<F>(
    reqwest_request: impl Fn() -> F,
) -> Result<reqwest::Response, reqwest::Error>
where
    F: Future<Output = Result<Response, reqwest::Error>>,
{
    let mut resp = reqwest_request().await;

    for attempt in 0..RETRY_COUNT {
        let wait = match &resp {
            // Microsoft's authentication services go down for a minute often enough
            // that giving up on the first failure makes signing in flaky, and makes a
            // token refresh fail for a reason that has nothing to do with the account
            Ok(res) if is_transient_status(res.status()) => {
                match retry_after(res) {
                    // Waiting out a long back-off would leave the app hanging, and
                    // the service has told us there is no point in trying sooner
                    Some(wait) if wait > MAX_RETRY_WAIT => break,
                    Some(wait) => wait,
                    None => backoff_wait(attempt),
                }
            }
            Err(err) if err.is_connect() || err.is_timeout() => {
                backoff_wait(attempt)
            }
            _ => break,
        };

        tracing::debug!("Authentication request failed, retrying in {wait:?}");
        tokio::time::sleep(wait).await;
        resp = reqwest_request().await;
    }

    resp
}

pub struct DeviceTokenKey {
    pub id: Uuid,
    pub key: SigningKey,
    pub x: String,
    pub y: String,
}

#[tracing::instrument]
fn generate_key() -> Result<DeviceTokenKey, MinecraftAuthenticationError> {
    let uuid = Uuid::new_v4();

    let signing_key = SigningKey::random(&mut OsRng);
    let public_key = VerifyingKey::from(&signing_key);

    let encoded_point = public_key.to_encoded_point(false);

    Ok(DeviceTokenKey {
        id: uuid,
        key: signing_key,
        x: BASE64_URL_SAFE_NO_PAD.encode(
            encoded_point.x().ok_or_else(|| {
                MinecraftAuthenticationError::ReadingPublicKey
            })?,
        ),
        y: BASE64_URL_SAFE_NO_PAD.encode(
            encoded_point.y().ok_or_else(|| {
                MinecraftAuthenticationError::ReadingPublicKey
            })?,
        ),
    })
}

struct SignedRequestResponse<T> {
    pub headers: HeaderMap,
    pub current_date: DateTime<Utc>,
    pub body: T,
}

#[tracing::instrument(skip(key))]
async fn send_signed_request<T: DeserializeOwned>(
    authorization: Option<&str>,
    url: &str,
    url_path: &str,
    raw_body: serde_json::Value,
    key: &DeviceTokenKey,
    step: MinecraftAuthStep,
    current_date: DateTime<Utc>,
) -> Result<SignedRequestResponse<T>, MinecraftAuthenticationError> {
    let auth = authorization.map_or(Vec::new(), |v| v.as_bytes().to_vec());

    let body = serde_json::to_vec(&raw_body).map_err(|source| {
        MinecraftAuthenticationError::SerializeBody { source, step }
    })?;
    let time: u128 =
        { ((current_date.timestamp() as u128) + 11644473600) * 10000000 };

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&1_u32.to_be_bytes()[..]);
    buffer.push(0_u8);
    buffer.extend_from_slice(&(time as u64).to_be_bytes()[..]);
    buffer.push(0_u8);
    buffer.extend_from_slice("POST".as_bytes());
    buffer.push(0_u8);
    buffer.extend_from_slice(url_path.as_bytes());
    buffer.push(0_u8);
    buffer.extend_from_slice(&auth);
    buffer.push(0_u8);
    buffer.extend_from_slice(&body);
    buffer.push(0_u8);

    let ecdsa_sig: Signature = key.key.sign(&buffer);

    let mut sig_buffer = Vec::new();
    sig_buffer.extend_from_slice(&1_i32.to_be_bytes()[..]);
    sig_buffer.extend_from_slice(&(time as u64).to_be_bytes()[..]);
    sig_buffer.extend_from_slice(&ecdsa_sig.r().to_bytes());
    sig_buffer.extend_from_slice(&ecdsa_sig.s().to_bytes());

    let signature = BASE64_STANDARD.encode(&sig_buffer);

    let res = auth_retry(|| {
        let mut request = INSECURE_REQWEST_CLIENT
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Accept", "application/json")
            .header("Signature", &signature);

        if url != "https://sisu.xboxlive.com/authorize" {
            request = request.header("x-xbl-contract-version", "1");
        }

        if let Some(auth) = authorization {
            request = request.header("Authorization", auth);
        }

        request.body(body.clone()).send()
    })
    .await
    .map_err(|source| MinecraftAuthenticationError::Request { source, step })?;

    let status = res.status();
    let headers = res.headers().clone();

    let current_date = get_date_header(&headers);

    let body = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request { source, step }
    })?;

    let body = parse_auth_response(body, status, step)?;
    Ok(SignedRequestResponse {
        headers,
        current_date,
        body,
    })
}

#[tracing::instrument]
fn get_date_header(headers: &HeaderMap) -> DateTime<Utc> {
    headers
        .get(reqwest::header::DATE)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| DateTime::parse_from_rfc2822(x).ok())
        .map_or(Utc::now(), |x| x.with_timezone(&Utc))
}

#[tracing::instrument]
fn generate_oauth_challenge() -> String {
    let mut rng = rand::thread_rng();

    let bytes: Vec<u8> = (0..64).map(|_| rng.r#gen::<u8>()).collect();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
