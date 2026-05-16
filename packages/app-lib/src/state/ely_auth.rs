use crate::util::fetch::INSECURE_REQWEST_CLIENT;
use serde::{Deserialize, Serialize};
use serde::ser::SerializeStruct;
use serde::Serializer;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct ElyCredentials {
    pub uuid: Uuid,
    pub username: String,
    pub access_token: String,
    pub client_token: String,
    pub active: bool,
}

impl Serialize for ElyCredentials {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ElyCredentials", 4)?;
        s.serialize_field("profile", &serde_json::json!({
            "id": self.uuid,
            "name": self.username,
            "skins": [],
            "capes": []
        }))?;
        s.serialize_field("access_token", &self.access_token)?;
        s.serialize_field("active", &self.active)?;
        s.serialize_field("auth_provider", "ely_by")?;
        s.end()
    }
}

#[derive(Deserialize)]
struct ElyAuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "clientToken")]
    client_token: String,
    #[serde(rename = "selectedProfile")]
    selected_profile: ElyProfile,
}

#[derive(Deserialize)]
struct ElyProfile {
    id: String,
    name: String,
}

impl ElyCredentials {
    pub async fn authenticate(username: &str, password: &str) -> crate::Result<Self> {
        let client_token = Uuid::new_v4().to_string();

        let resp = INSECURE_REQWEST_CLIENT
            .post("https://authserver.ely.by/auth/authenticate")
            .json(&serde_json::json!({
                "username": username,
                "password": password,
                "clientToken": client_token,
                "requestUser": true,
                "agent": { "name": "Minecraft", "version": 1 }
            }))
            .send()
            .await
            .map_err(|e| crate::ErrorKind::OtherError(format!("Ely.by auth request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await
            .map_err(|e| crate::ErrorKind::OtherError(format!("Failed to read Ely.by response: {e}")))?;

        if !status.is_success() {
            // Try to extract error message
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["errorMessage"].as_str().map(String::from))
                .unwrap_or_else(|| format!("HTTP {status}"));
            return Err(crate::ErrorKind::OtherError(format!("Ely.by login failed: {msg}")).into());
        }

        let data: ElyAuthResponse = serde_json::from_str(&body)
            .map_err(|e| crate::ErrorKind::OtherError(format!("Failed to parse Ely.by auth response: {e}. Body: {body}")))?;

        let uuid = Uuid::parse_str(&data.selected_profile.id)
            .map_err(|e| crate::ErrorKind::OtherError(format!("Invalid UUID from Ely.by: {e}")))?;

        Ok(Self {
            uuid,
            username: data.selected_profile.name,
            access_token: data.access_token,
            client_token: data.client_token,
            active: true,
        })
    }

    async fn validate(&self) -> bool {
        let resp = INSECURE_REQWEST_CLIENT
            .post("https://authserver.ely.by/auth/validate")
            .json(&serde_json::json!({
                "accessToken": self.access_token,
                "clientToken": self.client_token
            }))
            .send()
            .await;

        matches!(resp, Ok(r) if r.status().as_u16() == 204)
    }

    async fn refresh(&mut self) -> crate::Result<()> {
        let resp = INSECURE_REQWEST_CLIENT
            .post("https://authserver.ely.by/auth/refresh")
            .json(&serde_json::json!({
                "accessToken": self.access_token,
                "clientToken": self.client_token
            }))
            .send()
            .await
            .map_err(|e| crate::ErrorKind::OtherError(format!("Ely.by refresh request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await
            .map_err(|e| crate::ErrorKind::OtherError(format!("Failed to read Ely.by refresh response: {e}")))?;

        if !status.is_success() {
            return Err(crate::ErrorKind::OtherError(format!("Ely.by token refresh failed: HTTP {status}")).into());
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            #[serde(rename = "accessToken")]
            access_token: String,
        }

        let data: RefreshResponse = serde_json::from_str(&body)
            .map_err(|e| crate::ErrorKind::OtherError(format!("Failed to parse Ely.by refresh response: {e}")))?;

        self.access_token = data.access_token;
        Ok(())
    }

    pub async fn get_active(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Option<Self>> {
        let res = sqlx::query!(
            "SELECT uuid, active, username, access_token, client_token FROM ely_users WHERE active = TRUE"
        )
        .fetch_optional(exec)
        .await?;

        let Some(row) = res else { return Ok(None) };

        let uuid = Uuid::parse_str(&row.uuid)
            .map_err(|e| crate::ErrorKind::OtherError(format!("Invalid UUID in ely_users: {e}")))?;

        let mut creds = Self {
            uuid,
            username: row.username,
            access_token: row.access_token,
            client_token: row.client_token,
            active: row.active == 1,
        };

        // Validate and refresh if needed
        if !creds.validate().await {
            match creds.refresh().await {
                Ok(()) => {
                    creds.upsert(exec).await.ok();
                }
                Err(_) => {
                    // Token dead, remove
                    Self::remove(creds.uuid, exec).await.ok();
                    return Ok(None);
                }
            }
        }

        Ok(Some(creds))
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Vec<Self>> {
        let rows = sqlx::query!(
            "SELECT uuid, active, username, access_token, client_token FROM ely_users"
        )
        .fetch_all(exec)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let Ok(uuid) = Uuid::parse_str(&row.uuid) else { continue };
            result.push(Self {
                uuid,
                username: row.username,
                access_token: row.access_token,
                client_token: row.client_token,
                active: row.active == 1,
            });
        }

        Ok(result)
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        let uuid = self.uuid.as_hyphenated().to_string();

        if self.active {
            sqlx::query!("UPDATE ely_users SET active = FALSE")
                .execute(exec)
                .await?;
        }

        sqlx::query!(
            "INSERT INTO ely_users (uuid, active, username, access_token, client_token)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (uuid) DO UPDATE SET
                 active = $2, username = $3, access_token = $4, client_token = $5",
            uuid,
            self.active,
            self.username,
            self.access_token,
            self.client_token,
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
        sqlx::query!("DELETE FROM ely_users WHERE uuid = $1", uuid)
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Builds a Minecraft [`Credentials`](crate::state::Credentials) object
    /// from this Ely.by account so it can be passed through the regular
    /// launch pipeline. The game must additionally be launched with the
    /// authlib-injector Java agent for the token to be accepted.
    pub fn to_minecraft_credentials(&self) -> crate::state::Credentials {
        crate::state::Credentials {
            offline_profile: crate::state::MinecraftProfile {
                id: self.uuid,
                name: self.username.clone(),
                ..Default::default()
            },
            access_token: self.access_token.clone(),
            // Ely.by tokens are refreshed via the Ely.by flow, not the
            // Microsoft one — set a far-future expiry so the Microsoft
            // refresh path is never triggered for this synthetic object.
            refresh_token: String::new(),
            expires: chrono::Utc::now() + chrono::Duration::weeks(52),
            active: true,
        }
    }
}
