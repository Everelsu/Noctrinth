use crate::state::ElyCredentials;
use uuid::Uuid;

pub async fn login(
    username: &str,
    password: &str,
) -> crate::Result<ElyCredentials> {
    let state = crate::State::get().await?;
    let creds = ElyCredentials::authenticate(username, password).await?;
    creds.upsert(&state.pool).await?;
    Ok(creds)
}

pub async fn logout(uuid: Uuid) -> crate::Result<()> {
    let state = crate::State::get().await?;
    ElyCredentials::remove(uuid, &state.pool).await
}

pub async fn get_default_user() -> crate::Result<Option<Uuid>> {
    let state = crate::State::get().await?;
    let user = ElyCredentials::get_active(&state.pool).await?;
    Ok(user.map(|u| u.uuid))
}

pub async fn set_default_user(uuid: Uuid) -> crate::Result<()> {
    let state = crate::State::get().await?;
    let all = ElyCredentials::get_all(&state.pool).await?;

    // Deactivate all Ely.by accounts.
    sqlx::query!("UPDATE ely_users SET active = FALSE")
        .execute(&state.pool)
        .await
        .map_err(crate::Error::from)?;

    // Deactivate all Microsoft accounts too: there is exactly one active
    // account across both providers, so picking an Ely.by account must
    // unselect any Microsoft account (and vice versa).
    sqlx::query!("UPDATE minecraft_users SET active = FALSE")
        .execute(&state.pool)
        .await
        .map_err(crate::Error::from)?;

    // Activate the selected one
    if let Some(mut user) = all.into_iter().find(|u| u.uuid == uuid) {
        user.active = true;
        user.upsert(&state.pool).await?;
    }

    Ok(())
}

pub async fn users() -> crate::Result<Vec<ElyCredentials>> {
    let state = crate::State::get().await?;
    ElyCredentials::get_all(&state.pool).await
}

/// Fetches the raw PNG skin texture for an Ely.by user from the public
/// skin system. Returns the PNG bytes, or an error if the user has no
/// custom skin (HTTP 404) or the request fails.
pub async fn get_skin_texture(username: &str) -> crate::Result<Vec<u8>> {
    use crate::util::fetch::INSECURE_REQWEST_CLIENT;

    // The timestamp query busts HTTP caches (CDN and client) so a freshly
    // uploaded skin shows up immediately instead of a stale cached texture.
    let url = format!(
        "https://skinsystem.ely.by/skins/{}.png?_={}",
        urlencoding::encode(username),
        chrono::Utc::now().timestamp_millis()
    );

    let resp = INSECURE_REQWEST_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to fetch Ely.by skin: {e}"
            ))
        })?;

    if !resp.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by skin not available (HTTP {})",
            resp.status()
        ))
        .into());
    }

    let bytes = resp.bytes().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read Ely.by skin data: {e}"
        ))
    })?;

    Ok(bytes.to_vec())
}

/// One skin the user has uploaded to Ely.by's public catalogue.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ElyUploadedSkin {
    /// The catalogue ID, which is also what `/skins/wear` takes.
    pub id: u64,
    /// Direct URL of the skin texture.
    pub skin_url: String,
    /// Whether the skin uses the three-pixel-arm model.
    pub is_slim: bool,
}

#[derive(serde::Deserialize)]
struct ElySkinListing {
    items: Vec<ElyUploadedSkin>,
}

/// Lists the skins a user has uploaded to Ely.by.
///
/// Ely.by has no skin API, but the website's own listing endpoint is public
/// and returns exactly what a skin grid needs — including the ID that
/// `/skins/wear` expects. It is fetched here rather than from the frontend
/// because ely.by sends no CORS headers.
pub async fn list_uploaded_skins(
    username: &str,
) -> crate::Result<Vec<ElyUploadedSkin>> {
    use crate::util::fetch::INSECURE_REQWEST_CLIENT;

    let url = format!(
        "https://ely.by/skins/get?uploader={}",
        urlencoding::encode(username)
    );

    let resp = INSECURE_REQWEST_CLIENT
        .get(&url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to list Ely.by skins: {e}"
            ))
        })?;

    if !resp.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by skin listing failed (HTTP {})",
            resp.status()
        ))
        .into());
    }

    let listing: ElySkinListing = resp.json().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Could not read the Ely.by skin listing: {e}"
        ))
    })?;

    Ok(listing.items)
}

#[derive(serde::Deserialize)]
struct ElyTexture {
    url: String,
}

#[derive(serde::Deserialize)]
struct ElyTextures {
    #[serde(rename = "SKIN")]
    skin: Option<ElyTexture>,
}

/// The storage URL of the skin a user is currently wearing.
///
/// The listing gives the same URLs, so comparing the two is what marks the
/// active skin in the grid. Returns `None` when the account wears the default
/// skin.
pub async fn get_current_skin_url(
    username: &str,
) -> crate::Result<Option<String>> {
    use crate::util::fetch::INSECURE_REQWEST_CLIENT;

    let url = format!(
        "https://skinsystem.ely.by/textures/{}",
        urlencoding::encode(username)
    );

    let resp = INSECURE_REQWEST_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to read the current Ely.by skin: {e}"
            ))
        })?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let textures: ElyTextures = match resp.json().await {
        Ok(textures) => textures,
        // No custom skin: the endpoint answers with something that is not a
        // texture set, which is not an error worth surfacing.
        Err(_) => return Ok(None),
    };

    Ok(textures.skin.map(|skin| skin.url))
}

/// Fetches a texture from Ely.by and returns the raw PNG bytes.
///
/// The frontend cannot fetch these itself: ely.by is not in the HTTP plugin's
/// allowlist, and were it added, the images still carry no CORS headers and
/// would taint the canvas the skin grid bakes previews on.
///
/// The URL comes from the frontend, so the host is checked here rather than
/// trusted — this must not become a general-purpose proxy.
pub async fn get_texture_bytes(url: &str) -> crate::Result<Vec<u8>> {
    use crate::util::fetch::INSECURE_REQWEST_CLIENT;

    let parsed = reqwest::Url::parse(url).map_err(|e| {
        crate::ErrorKind::InputError(format!("Not a texture URL: {e}"))
    })?;

    let host = parsed.host_str().unwrap_or_default();
    if host != "ely.by" && !host.ends_with(".ely.by") {
        return Err(crate::ErrorKind::InputError(format!(
            "Refusing to fetch a texture from {host}"
        ))
        .into());
    }

    let resp =
        INSECURE_REQWEST_CLIENT
            .get(parsed)
            .send()
            .await
            .map_err(|e| {
                crate::ErrorKind::OtherError(format!(
                    "Failed to fetch the Ely.by texture: {e}"
                ))
            })?;

    if !resp.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by texture not available (HTTP {})",
            resp.status()
        ))
        .into());
    }

    let bytes = resp.bytes().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read the Ely.by texture: {e}"
        ))
    })?;

    Ok(bytes.to_vec())
}
