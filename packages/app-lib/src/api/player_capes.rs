//! A player's cape, by name, from every service that hands them out.
//!
//! The game itself finds capes through the agent in `theseus.jar`, by the same
//! name and in the same order: the launcher's own folder, Ely.by, Mojang,
//! LittleSkin, OptiFine, then LabyMod, MinecraftCapes and SkinMC — capes their
//! own mods draw, which the agent hands to everyone else as well. The cape the
//! preview shows is the one the game shows.
//!
//! The services asked by id want the id Mojang issued, so they only answer for
//! a name somebody has bought; an offline name nobody owns has no cape there.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use image::{GenericImageView, ImageFormat, RgbaImage};
use serde::Serialize;

use crate::State;
use crate::util::fetch::REQWEST_CLIENT;

/// How long one service gets to answer before it is taken to have nothing.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);

/// How long an answer, cape or none, is reused.
const CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapeSource {
    Local,
    ElyBy,
    Mojang,
    LittleSkin,
    OptiFine,
    LabyMod,
    MinecraftCapes,
    SkinMc,
}

#[derive(Serialize, Debug, Clone)]
pub struct PlayerCape {
    pub source: CapeSource,
    /// A PNG laid out like the game's own capes: twice as wide as it is tall.
    pub png: Vec<u8>,
}

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Option<PlayerCape>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The first cape found for `name`, or `None` when nobody has one for it.
pub async fn cape_for_name(name: &str) -> crate::Result<Option<PlayerCape>> {
    let key = name.to_lowercase();
    if let Some((at, cape)) =
        CACHE.lock().ok().and_then(|cache| cache.get(&key).cloned())
        && at.elapsed() < CACHE_TTL
    {
        return Ok(cape);
    }

    let cape = find(name).await;
    if let Ok(mut cache) = CACHE.lock() {
        cache.insert(key, (Instant::now(), cape.clone()));
    }
    Ok(cape)
}

async fn find(name: &str) -> Option<PlayerCape> {
    if let Some(png) = local(name).await {
        return Some(PlayerCape {
            source: CapeSource::Local,
            png,
        });
    }

    let encoded = urlencoding::encode(name).into_owned();
    let mojang_id = mojang_id(name);
    let ely =
        get_png(format!("https://skinsystem.ely.by/cloaks/{encoded}.png"));
    let optifine = get_png(format!("https://optifine.net/capes/{encoded}.png"));
    let littleskin = littleskin_cape(&encoded);
    let (mojang_id, ely, optifine, littleskin) =
        tokio::join!(mojang_id, ely, optifine, littleskin);

    let (mojang, labymod, minecraft_capes, skinmc) = match &mojang_id {
        Some(id) => {
            let dashed = dashed(id);
            tokio::join!(
                mojang_cape(id),
                get_png(format!("https://dl.labymod.net/capes/{dashed}")),
                minecraft_capes_cape(id),
                get_png(format!(
                    "https://skinmc.net/api/v1/skinmcCape/{dashed}"
                )),
            )
        }
        None => (None, None, None, None),
    };

    [
        (CapeSource::ElyBy, ely),
        (CapeSource::Mojang, mojang),
        (CapeSource::LittleSkin, littleskin),
        (CapeSource::OptiFine, optifine),
        (CapeSource::LabyMod, labymod),
        (CapeSource::MinecraftCapes, minecraft_capes),
        (CapeSource::SkinMc, skinmc),
    ]
    .into_iter()
    .find_map(|(source, png)| {
        normalize(&png?).map(|png| PlayerCape { source, png })
    })
}

/// `capes/<name>.png` in the launcher's skins folder, the one the agent reads.
async fn local(name: &str) -> Option<Vec<u8>> {
    let state = State::get_if_initialized()?;
    let path = state
        .directories
        .player_skins_dir()
        .join("capes")
        .join(format!("{name}.png"));
    let png = tokio::fs::read(path).await.ok()?;
    normalize(&png)
}

/// A PNG at `url`, or `None` for anything else — a missing cape is a 404 on
/// every one of these services, and an error page is not a cape either.
async fn get_png(url: String) -> Option<Vec<u8>> {
    let response = REQWEST_CLIENT
        .get(&url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    bytes.starts_with(b"\x89PNG").then(|| bytes.to_vec())
}

/// The cape LittleSkin names for `name` in CustomSkinLoader's format.
async fn littleskin_cape(encoded_name: &str) -> Option<Vec<u8>> {
    let profile: serde_json::Value = REQWEST_CLIENT
        .get(format!("https://littleskin.cn/csl/{encoded_name}.json"))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let hash = profile
        .get("cape")?
        .as_str()
        .filter(|hash| !hash.is_empty())?;
    get_png(format!("https://littleskin.cn/textures/{hash}")).await
}

/// The cape a MinecraftCapes profile points at.
///
/// The service stopped serving the picture at `/profile/{id}/cape` and names
/// its address in the profile instead; an animated cape only has the second.
async fn minecraft_capes_cape(id: &str) -> Option<Vec<u8>> {
    let profile: serde_json::Value = REQWEST_CLIENT
        .get(format!("https://api.minecraftcapes.net/profile/{id}"))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let url =
        ["cape_url", "animated_cape_url"]
            .into_iter()
            .find_map(|key| {
                profile
                    .get(key)?
                    .as_str()
                    .filter(|url| !url.is_empty())
                    .map(str::to_string)
            })?;
    get_png(url).await
}

/// The id Mojang gave the account that owns `name`, without dashes.
async fn mojang_id(name: &str) -> Option<String> {
    let profile: serde_json::Value = REQWEST_CLIENT
        .get(format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            urlencoding::encode(name)
        ))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    profile.get("id")?.as_str().map(str::to_string)
}

/// The official cape on that account, from the textures its profile carries.
async fn mojang_cape(id: &str) -> Option<Vec<u8>> {
    use base64::Engine;

    let profile: serde_json::Value = REQWEST_CLIENT
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
        ))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let packed = profile
        .get("properties")?
        .as_array()?
        .iter()
        .find(|property| {
            property.get("name").and_then(|name| name.as_str())
                == Some("textures")
        })?
        .get("value")?
        .as_str()?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(packed)
        .ok()?;
    let textures: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let url = textures.pointer("/textures/CAPE/url")?.as_str()?;

    get_png(url.replace("http://", "https://")).await
}

fn dashed(id: &str) -> String {
    if id.len() != 32 {
        return id.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &id[0..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..32]
    )
}

/// The cape laid out the way the game's are: a canvas twice as wide as tall.
///
/// OptiFine's are 46 by 22 — the game's 64 by 32 with the empty part cut off —
/// and LabyMod's are the 22 by 17 of the cape alone; both are put back in the
/// corner of a full canvas. MinecraftCapes stacks the frames of an animated
/// cape one under another; the first frame is the cape.
fn normalize(png: &[u8]) -> Option<Vec<u8>> {
    let image =
        image::load_from_memory_with_format(png, ImageFormat::Png).ok()?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let corner_scale = [(46, 22), (22, 17)].into_iter().find_map(|(w, h)| {
        (width % w == 0 && height % h == 0 && width / w == height / h)
            .then_some(width / w)
    });
    let canvas = if let Some(scale) = corner_scale {
        let mut canvas = RgbaImage::new(64 * scale, 32 * scale);
        image::imageops::overlay(&mut canvas, &image.to_rgba8(), 0, 0);
        canvas
    } else if height > width / 2 && height % (width / 2) == 0 {
        image.crop_imm(0, 0, width, width / 2).to_rgba8()
    } else {
        image.to_rgba8()
    };

    let mut out = Cursor::new(Vec::new());
    canvas.write_to(&mut out, ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        RgbaImage::new(width, height)
            .write_to(&mut out, ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn size(png: &[u8]) -> (u32, u32) {
        image::load_from_memory(png).unwrap().dimensions()
    }

    #[test]
    fn an_optifine_cape_goes_back_on_a_full_canvas() {
        assert_eq!(size(&normalize(&png(46, 22)).unwrap()), (64, 32));
        assert_eq!(size(&normalize(&png(92, 44)).unwrap()), (128, 64));
    }

    #[test]
    fn a_labymod_cape_goes_back_on_a_full_canvas() {
        assert_eq!(size(&normalize(&png(22, 17)).unwrap()), (64, 32));
        assert_eq!(size(&normalize(&png(352, 272)).unwrap()), (1024, 512));
    }

    #[test]
    fn an_animated_cape_is_its_first_frame() {
        assert_eq!(size(&normalize(&png(64, 128)).unwrap()), (64, 32));
    }

    #[test]
    fn a_game_cape_is_left_as_it_is() {
        assert_eq!(size(&normalize(&png(64, 32)).unwrap()), (64, 32));
    }

    #[test]
    fn an_id_gets_its_dashes() {
        assert_eq!(
            dashed("069a79f444e94726a5befca90e38aaf5"),
            "069a79f4-44e9-4726-a5be-fca90e38aaf5"
        );
    }
}
