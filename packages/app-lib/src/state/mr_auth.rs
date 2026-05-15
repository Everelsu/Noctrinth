use crate::state::{CacheBehaviour, CachedEntry};
use crate::util::fetch::{FetchSemaphore, REQWEST_CLIENT, fetch_advanced};
use chrono::{DateTime, Duration, TimeZone, Utc};
use dashmap::DashMap;
use futures::TryStreamExt;
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModrinthCredentials {
    pub session: String,
    pub expires: DateTime<Utc>,
    pub user_id: String,
    pub active: bool,
}

impl ModrinthCredentials {
    pub async fn get_and_refresh(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
        semaphore: &FetchSemaphore,
    ) -> crate::Result<Option<Self>> {
        let creds = Self::get_active(exec).await?;

        if let Some(mut creds) = creds {
            if creds.expires < Utc::now() {
                // OAuth access tokens (mro_ prefix) cannot be refreshed — require re-auth
                if creds.session.starts_with("mro_") {
                    Self::remove(&creds.user_id, exec).await?;
                    return Ok(None);
                }

                #[derive(Deserialize)]
                struct Session {
                    session: String,
                }

                let resp = fetch_advanced(
                    Method::POST,
                    concat!(env!("MODRINTH_API_URL"), "session/refresh"),
                    None,
                    None,
                    Some(("Authorization", &*creds.session)),
                    None,
                    None,
                    semaphore,
                    exec,
                )
                .await
                .ok()
                .and_then(|resp| serde_json::from_slice::<Session>(&resp).ok());

                if let Some(value) = resp {
                    creds.session = value.session;
                    creds.expires = Utc::now() + Duration::weeks(2);
                    creds.upsert(exec).await?;

                    Ok(Some(creds))
                } else {
                    Self::remove(&creds.user_id, exec).await?;

                    Ok(None)
                }
            } else {
                Ok(Some(creds))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_active(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
        let res = sqlx::query!(
            "
            SELECT
                id, active, session_id, expires
            FROM modrinth_users
            WHERE active = TRUE
            "
        )
        .fetch_optional(exec)
        .await?;

        Ok(res.map(|x| Self {
            session: x.session_id,
            expires: Utc
                .timestamp_opt(x.expires, 0)
                .single()
                .unwrap_or_else(Utc::now),
            user_id: x.id,
            active: x.active == 1,
        }))
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<DashMap<String, Self>> {
        let res = sqlx::query!(
            "
            SELECT
                id, active, session_id, expires
            FROM modrinth_users
            "
        )
        .fetch(exec)
        .try_fold(DashMap::new(), |acc, x| {
            acc.insert(
                x.id.clone(),
                Self {
                    session: x.session_id,
                    expires: Utc
                        .timestamp_opt(x.expires, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                    user_id: x.id,
                    active: x.active == 1,
                },
            );

            async move { Ok(acc) }
        })
        .await?;

        Ok(res)
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        let expires = self.expires.timestamp();

        if self.active {
            sqlx::query!(
                "
                UPDATE modrinth_users
                SET active = FALSE
                "
            )
            .execute(exec)
            .await?;
        }

        sqlx::query!(
            "
            INSERT INTO modrinth_users (id, active, session_id, expires)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                active = $2,
                session_id = $3,
                expires = $4
            ",
            self.user_id,
            self.active,
            self.session,
            expires,
        )
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove(
        user_id: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query!(
            "
            DELETE FROM modrinth_users WHERE id = $1
            ",
            user_id,
        )
        .execute(exec)
        .await?;

        Ok(())
    }

    pub(crate) async fn refresh_all() -> crate::Result<()> {
        let state = crate::State::get().await?;
        let all = Self::get_all(&state.pool).await?;

        let user_ids = all.into_iter().map(|x| x.0).collect::<Vec<_>>();

        CachedEntry::get_user_many(
            &user_ids.iter().map(|x| &**x).collect::<Vec<_>>(),
            Some(CacheBehaviour::Bypass),
            &state.pool,
            &state.fetch_semaphore,
        )
        .await?;

        Ok(())
    }
}

const OAUTH_SCOPES: &str =
    "USER_READ USER_READ_EMAIL USER_WRITE NOTIFICATION_READ NOTIFICATION_WRITE \
     COLLECTION_READ COLLECTION_WRITE COLLECTION_CREATE COLLECTION_DELETE";

pub fn build_login_url(redirect_uri: &str) -> String {
    format!(
        "{}auth/authorize?client_id={}&redirect_uri={}&scope={}&response_type=code",
        env!("MODRINTH_URL"),
        env!("MODRINTH_OAUTH_CLIENT_ID"),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(OAUTH_SCOPES),
    )
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: i64,
}

pub async fn exchange_code_for_token(
    code: &str,
    redirect_uri: &str,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<ModrinthCredentials> {
    let response = REQWEST_CLIENT
        .post(concat!(env!("MODRINTH_API_BASE_URL"), "_internal/oauth/token"))
        .header("Authorization", env!("MODRINTH_OAUTH_CLIENT_SECRET"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", env!("MODRINTH_OAUTH_CLIENT_ID")),
        ])
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Modrinth OAuth token request failed: {e}"
            ))
        })?;

    let status = response.status();
    let body = response.text().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read Modrinth OAuth token response: {e}"
        ))
    })?;

    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Modrinth OAuth token exchange failed (HTTP {status}): {body}"
        ))
        .into());
    }

    let token_resp =
        serde_json::from_str::<OAuthTokenResponse>(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to parse Modrinth OAuth token response: {e}. Body: {body}"
            ))
        })?;

    let info =
        fetch_info(&token_resp.access_token, semaphore, exec).await?;

    Ok(ModrinthCredentials {
        session: token_resp.access_token,
        expires: Utc::now() + Duration::seconds(token_resp.expires_in),
        user_id: info.id,
        active: true,
    })
}

async fn fetch_info(
    token: &str,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<crate::state::cache::User> {
    let result = fetch_advanced(
        Method::GET,
        concat!(env!("MODRINTH_API_URL"), "user"),
        None,
        None,
        Some(("Authorization", token)),
        None,
        None,
        semaphore,
        exec,
    )
    .await?;
    let value = serde_json::from_slice(&result)?;

    Ok(value)
}
