use std::path::PathBuf;

use crate::{
    ErrorKind,
    event::{
        CommandPayload,
        emit::{emit_command, emit_warning},
    },
    util::io,
};
use anyhow::{Context, bail};
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

/// CurseForge's scheme, which its site's Install button opens. The app takes
/// it only when asked to, in the CurseForge settings, since it belongs to
/// CurseForge's own app.
pub const CURSEFORGE_DEEP_LINK_SCHEME: &str = "curseforge";

/// `curseforge://install?addonId=…&fileId=…`. Both ids are CurseForge's
/// numbers; anything else is not an install link of theirs.
fn curseforge_install_link(command_string: &str) -> Option<CommandPayload> {
    let url = url::Url::parse(command_string).ok()?;
    if !url.scheme().eq_ignore_ascii_case(CURSEFORGE_DEEP_LINK_SCHEME)
        || url.host_str() != Some("install")
    {
        return None;
    }
    let number = |value: &str| {
        (!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| value.to_string())
    };
    let mut project_id = None;
    let mut file_id = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "addonId" => project_id = number(&value),
            "fileId" => file_id = number(&value),
            _ => {}
        }
    }
    Some(CommandPayload::OpenCurseforgeProject {
        project_id: project_id?,
        file_id,
    })
}

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
                    return Err(e).context(ErrorKind::InputError(
                        "invalid UTF-8 in shared instance invite path"
                            .to_string(),
                    ));
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
                bail!(ErrorKind::InputError(
                    "cannot launch both a server and a singleplayer world"
                        .to_string(),
                ));
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
                    return Err(e).context(ErrorKind::InputError(
                        "invalid UTF-8 in instance path".to_string(),
                    ));
                }
            }
        }
        _ => {
            emit_warning(&format!(
                "Invalid command, unrecognized path: {sublink}"
            ))
            .await?;
            bail!(ErrorKind::InputError(format!(
                "invalid command, unrecognized path {sublink}"
            )));
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

    if let Some(command) = curseforge_install_link(command_string) {
        return Ok(command);
    }

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
            return Ok(CommandPayload::RunMRPack {
                path: path.to_string_lossy().into_owned(),
            });
        }
        emit_warning(&format!(
            "Invalid command, unrecognized filetype: {}",
            path.display()
        ))
        .await?;
        bail!(ErrorKind::InputError(format!(
            "invalid command, unrecognized filetype {}",
            path.display()
        )))
    }
}

pub async fn parse_and_emit_command(command_string: &str) -> crate::Result<()> {
    let command = parse_command(command_string).await?;
    emit_command(command).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_curseforge_install_links() {
        let Some(CommandPayload::OpenCurseforgeProject {
            project_id,
            file_id,
        }) = curseforge_install_link(
            "curseforge://install?addonId=238222&fileId=5101366",
        )
        else {
            panic!("not read as an install link");
        };
        assert_eq!(project_id, "238222");
        assert_eq!(file_id.as_deref(), Some("5101366"));

        assert!(matches!(
            curseforge_install_link("curseforge://install?addonId=238222"),
            Some(CommandPayload::OpenCurseforgeProject { file_id: None, .. })
        ));
        assert!(
            curseforge_install_link("curseforge://install?addonId=../x")
                .is_none()
        );
        assert!(
            curseforge_install_link("curseforge://open?addonId=1").is_none()
        );
        assert!(curseforge_install_link("noctrinth://mod/sodium").is_none());
    }
}
