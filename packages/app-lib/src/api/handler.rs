use std::path::PathBuf;

use crate::{
    event::{
        CommandPayload,
        emit::{emit_command, emit_warning},
    },
    util::io,
};
use url::form_urlencoded;
use urlencoding::decode;

/// This launcher's own URL scheme, used for every link we generate ourselves.
///
/// Kept in sync with `deep-link.desktop.schemes` in `apps/app/tauri.conf.json`
/// and `APP_DEEP_LINK_SCHEME` in `packages/ui/src/utils/deep-link.ts`.
pub const DEEP_LINK_SCHEME: &str = "noctrinth";

/// Modrinth's scheme. The app registers it as well, so install links on
/// modrinth.com open Noctrinth — the whole point of a launcher that speaks the
/// same API. Older shortcuts written before the rename land here too.
const MODRINTH_DEEP_LINK_SCHEME: &str = "modrinth";

/// Handles external functions (such as through URL deep linkage)
/// Link is extracted value (link) in somewhat URL format, such as
/// subdomain1/subdomain2
/// (Does not include the `noctrinth://` prefix)
pub async fn handle_url(sublink: &str) -> crate::Result<CommandPayload> {
    Ok(match sublink.split_once('/') {
        // /mod/{id}   -    Installs a mod of mod id
        Some(("mod", id)) => CommandPayload::InstallMod { id: id.to_string() },
        // /version/{id}   -    Installs a specific version of id
        Some(("version", id)) => {
            CommandPayload::InstallVersion { id: id.to_string() }
        }
        // /modpack/{id}   -    Installs a modpack of modpack id
        Some(("modpack", id)) => {
            CommandPayload::InstallModpack { id: id.to_string() }
        }
        // /server/{id}   -    Opens a server project page and triggers play flow
        Some(("server", id)) => {
            CommandPayload::InstallServer { id: id.to_string() }
        }
        // /share/{invite_id}
        Some(("share", raw)) => {
            let (raw, _) = raw.split_once('?').unwrap_or((raw, ""));

            match decode(raw) {
                Ok(decoded) => CommandPayload::InstallSharedInstanceInvite {
                    invite_id: decoded.to_string(),
                },
                Err(e) => {
                    emit_warning(&format!(
                        "Invalid UTF-8 in shared instance invite path: {e}"
                    ))
                    .await?;
                    return Err(crate::ErrorKind::InputError(format!(
                        "Invalid UTF-8 in shared instance invite path: {e}"
                    ))
                    .into());
                }
            }
        }
        // /launch/instance/{id}   -    Launches an instance
        Some(("launch", rest)) if rest.starts_with("instance/") => {
            let raw = rest.trim_start_matches("instance/");
            let (raw, query) = raw.split_once('?').unwrap_or((raw, ""));
            let mut server = None;
            let mut singleplayer_world = None;

            for (key, value) in form_urlencoded::parse(query.as_bytes()) {
                match &*key {
                    "server" => server = Some(value.into_owned()),
                    "singleplayer_world" => {
                        singleplayer_world = Some(value.into_owned());
                    }
                    _ => {}
                }
            }

            if server.is_some() && singleplayer_world.is_some() {
                emit_warning(
                    "Invalid command, cannot launch both a server and a singleplayer world",
                )
                .await?;
                return Err(crate::ErrorKind::InputError(
                    "Cannot launch both a server and a singleplayer world"
                        .to_string(),
                )
                .into());
            }

            match decode(raw) {
                Ok(decoded) => CommandPayload::LaunchInstance {
                    id: decoded.to_string(),
                    server,
                    singleplayer_world,
                },
                Err(e) => {
                    emit_warning(&format!(
                        "Invalid UTF-8 in instance path: {e}"
                    ))
                    .await?;
                    return Err(crate::ErrorKind::InputError(format!(
                        "Invalid UTF-8 in instance path: {e}"
                    ))
                    .into());
                }
            }
        }
        _ => {
            emit_warning(&format!(
                "Invalid command, unrecognized path: {sublink}"
            ))
            .await?;
            return Err(crate::ErrorKind::InputError(format!(
                "Invalid command, unrecognized path: {sublink}"
            ))
            .into());
        }
    })
}

/// Strips a recognised deep-link scheme, returning the command that follows it.
///
/// Schemes are compared case-insensitively: Windows hands the registered scheme
/// back in whatever case the link was written in.
fn strip_deep_link_scheme(command_string: &str) -> Option<&str> {
    let (scheme, rest) = command_string.split_once("://")?;
    (scheme.eq_ignore_ascii_case(DEEP_LINK_SCHEME)
        || scheme.eq_ignore_ascii_case(MODRINTH_DEEP_LINK_SCHEME))
    .then_some(rest)
}

pub async fn parse_command(
    command_string: &str,
) -> crate::Result<CommandPayload> {
    tracing::debug!("Parsing external command");

    // noctrinth://some-command
    // This occurs when following a web redirect link
    if let Some(sublink) = strip_deep_link_scheme(command_string) {
        Ok(handle_url(sublink).await?)
    } else {
        // We assume anything else is a filepath to an .mrpack file
        let path = PathBuf::from(command_string);
        let path = io::canonicalize(path)?;
        if let Some(ext) = path.extension()
            && ext == "mrpack"
        {
            return Ok(CommandPayload::RunMRPack { path });
        }
        emit_warning(&format!(
            "Invalid command, unrecognized filetype: {}",
            path.display()
        ))
        .await?;
        Err(crate::ErrorKind::InputError(format!(
            "Invalid command, unrecognized filetype: {}",
            path.display()
        ))
        .into())
    }
}

pub async fn parse_and_emit_command(command_string: &str) -> crate::Result<()> {
    let command = parse_command(command_string).await?;
    emit_command(command).await?;
    Ok(())
}
