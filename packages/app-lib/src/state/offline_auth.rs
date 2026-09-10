//! Accounts that are a name and nothing else.
//!
//! The game does not need an account system to start. Singleplayer, a world
//! opened to LAN, and any server running in offline mode ask for a name and the
//! UUID that goes with it, and neither of those has ever been checked with
//! anybody. Every other launcher has offered this for years, under one name or
//! another; this is the fork's.
//!
//! It is what the launcher has left when there is no connection at all — the
//! Microsoft sign-in cannot be refreshed, Ely.by cannot be asked, and the
//! player would like to play. It is also the only way to launch on a machine
//! that has never been online.
//!
//! What it cannot do is join a server in online mode. Those ask Mojang whether
//! the session is real, and there is no session here. That is the whole of the
//! trade.

use chrono::{DateTime, TimeZone, Utc};
use md5::{Digest, Md5};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use uuid::Uuid;

/// What Minecraft calls a player who never signed in.
///
/// A server in offline mode derives the player's UUID from their name this
/// way — `UUID.nameUUIDFromBytes(("OfflinePlayer:" + name).getBytes(UTF_8))` —
/// and a launcher that used any other UUID would hand back a different player
/// each time: another inventory, another home, another set of permissions. So
/// it is derived exactly as the server does it.
const OFFLINE_PREFIX: &str = "OfflinePlayer:";

/// The shortest and longest a name Mojang would have issued can be.
const NAME_LENGTH: std::ops::RangeInclusive<usize> = 3..=16;

#[derive(Debug, Clone)]
pub struct OfflineCredentials {
    pub uuid: Uuid,
    pub username: String,
    pub active: bool,
    pub created: DateTime<Utc>,
}

impl Serialize for OfflineCredentials {
    /// The shape the interface reads every account in, so that an offline one
    /// can sit in the same list as the rest.
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut account =
            serializer.serialize_struct("OfflineCredentials", 4)?;
        account.serialize_field(
            "profile",
            &serde_json::json!({
                "id": self.uuid,
                "name": self.username,
                "skins": [],
                "capes": []
            }),
        )?;
        account.serialize_field("active", &self.active)?;
        account.serialize_field("auth_provider", "offline")?;
        account.serialize_field("created", &self.created)?;
        account.end()
    }
}

impl OfflineCredentials {
    /// The UUID an offline-mode server will know this name by.
    pub fn uuid_for(username: &str) -> Uuid {
        let mut digest = Md5::new();
        digest.update(format!("{OFFLINE_PREFIX}{username}").as_bytes());

        let mut bytes: [u8; 16] = digest.finalize().into();

        // Java's `nameUUIDFromBytes` stamps the digest as a version 3 UUID,
        // and the value only matches if it is stamped the same way.
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        Uuid::from_bytes(bytes)
    }

    /// Whether the game and its servers would accept this as a name.
    ///
    /// Not a rule of this launcher's: a server in offline mode turns away
    /// anything Mojang would not have issued, so a name that fails here would
    /// be a login that fails there.
    pub fn is_valid_username(username: &str) -> bool {
        NAME_LENGTH.contains(&username.chars().count())
            && username.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_'
            })
    }

    /// Adds the account, or renames nothing: a name is its own identity here,
    /// so adding one twice is adding it once.
    pub async fn create(
        username: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Self> {
        let username = username.trim();
        if !Self::is_valid_username(username) {
            return Err(crate::ErrorKind::InputError(
                "A Minecraft name is 3 to 16 letters, digits or underscores"
                    .to_string(),
            )
            .into());
        }

        let account = Self {
            uuid: Self::uuid_for(username),
            username: username.to_string(),
            active: true,
            created: Utc::now(),
        };

        account.upsert(exec).await?;
        Self::set_active(account.uuid, exec).await?;

        Ok(account)
    }

    async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query(
            "
            INSERT INTO offline_users (uuid, username, active, created)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (uuid) DO UPDATE SET username = excluded.username
            ",
        )
        .bind(self.uuid.as_hyphenated().to_string())
        .bind(&self.username)
        .bind(i64::from(self.active))
        .bind(self.created.timestamp())
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "
            SELECT uuid, username, active, created
            FROM offline_users
            ORDER BY created
            ",
        )
        .fetch_all(exec)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|(uuid, username, active, created)| {
                Some(Self {
                    uuid: Uuid::parse_str(&uuid).ok()?,
                    username,
                    active: active == 1,
                    created: Utc
                        .timestamp_opt(created, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                })
            })
            .collect())
    }

    /// The offline account the launcher would play as, if it came to that.
    pub async fn get_active(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
        Ok(Self::get_all(exec)
            .await?
            .into_iter()
            .find(|account| account.active))
    }

    pub async fn set_active(
        uuid: Uuid,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        sqlx::query("UPDATE offline_users SET active = FALSE")
            .execute(exec)
            .await?;

        sqlx::query("UPDATE offline_users SET active = TRUE WHERE uuid = ?")
            .bind(uuid.as_hyphenated().to_string())
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn remove(
        uuid: Uuid,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query("DELETE FROM offline_users WHERE uuid = ?")
            .bind(uuid.as_hyphenated().to_string())
            .execute(exec)
            .await?;

        Ok(())
    }

    /// The account as the launch path takes it.
    ///
    /// The token is the one every launcher hands the game when there is nothing
    /// to hand it. The game starts, the world loads, and the first server that
    /// asks Mojang about it turns the player away — which is what offline
    /// means. The expiry is far off because nothing can refresh this, and a
    /// past one would send the launcher looking for a Microsoft refresh that is
    /// not there.
    pub fn to_minecraft_credentials(&self) -> crate::state::Credentials {
        crate::state::Credentials {
            offline_profile: crate::state::MinecraftProfile {
                id: self.uuid,
                name: self.username.clone(),
                ..Default::default()
            },
            access_token: "0".to_string(),
            refresh_token: String::new(),
            expires: Utc::now() + chrono::Duration::weeks(52),
            active: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uuid_is_the_one_an_offline_server_would_use() {
        // What `UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes())` is,
        // and what every server in offline mode knows that name by.
        assert_eq!(
            OfflineCredentials::uuid_for("Notch")
                .as_hyphenated()
                .to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(
            OfflineCredentials::uuid_for("jeb_")
                .as_hyphenated()
                .to_string(),
            "a762f560-4fce-3236-812a-b80efff0b62b"
        );
    }

    #[test]
    fn the_same_name_is_always_the_same_player() {
        assert_eq!(
            OfflineCredentials::uuid_for("Steve"),
            OfflineCredentials::uuid_for("Steve")
        );
        assert_ne!(
            OfflineCredentials::uuid_for("Steve"),
            OfflineCredentials::uuid_for("steve")
        );
    }

    #[test]
    fn names_a_server_would_turn_away_are_turned_away_here() {
        assert!(OfflineCredentials::is_valid_username("Notch"));
        assert!(OfflineCredentials::is_valid_username("a_b_c"));
        assert!(OfflineCredentials::is_valid_username("Everesu123456789"));

        assert!(!OfflineCredentials::is_valid_username("no"));
        assert!(!OfflineCredentials::is_valid_username("seventeen_letters!"));
        assert!(!OfflineCredentials::is_valid_username("with space"));
        assert!(!OfflineCredentials::is_valid_username("Ярослав"));
        assert!(!OfflineCredentials::is_valid_username(""));
    }

    #[test]
    fn the_credentials_it_launches_with_are_its_own_name() {
        let account = OfflineCredentials {
            uuid: OfflineCredentials::uuid_for("Everesu"),
            username: "Everesu".to_string(),
            active: true,
            created: Utc::now(),
        };
        let credentials = account.to_minecraft_credentials();

        assert_eq!(credentials.offline_profile.name, "Everesu");
        assert_eq!(credentials.offline_profile.id, account.uuid);
        assert!(credentials.expires > Utc::now());
    }
}
