//! Theseus settings file

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

// Types
/// Global Theseus settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub max_concurrent_downloads: usize,
    pub max_concurrent_writes: usize,

    pub theme: Theme,
    pub locale: String,
    pub default_page: DefaultPage,
    pub collapsed_navigation: bool,
    pub hide_nametag_skins_page: bool,
    pub advanced_rendering: bool,
    pub native_decorations: bool,
    pub toggle_sidebar: bool,

    pub telemetry: bool,
    pub discord_rpc: bool,
    pub personalized_ads: bool,

    pub extra_launch_args: Vec<String>,
    pub custom_env_vars: Vec<(String, String)>,
    pub memory: MemorySettings,
    pub force_fullscreen: bool,
    pub game_resolution: WindowSize,
    pub hide_on_process_start: bool,
    pub hooks: Hooks,

    pub custom_dir: Option<String>,
    pub prev_custom_dir: Option<String>,
    pub migrated: bool,

    pub developer_mode: bool,
    pub feature_flags: HashMap<FeatureFlag, bool>,

    pub skipped_update: Option<String>,
    pub pending_update_toast_for_version: Option<String>,
    pub auto_download_updates: Option<bool>,

    /// Proxy URL applied to all launcher HTTP traffic (e.g.
    /// `socks5://127.0.0.1:1080` or `http://host:port`). Requires an app
    /// restart to take effect because reqwest clients are built once.
    pub proxy_url: Option<String>,

    /// Minecraft `options.txt` values applied to every instance at launch.
    pub shared_game_options: SharedGameOptions,

    pub version: usize,
}

/// A profile of `options.txt` entries shared across all instances.
///
/// The launcher already rewrites `options.txt` on launch for `fullscreen`, so
/// this rides the same path — see [`crate::launcher::launch_minecraft`].
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SharedGameOptions {
    pub enabled: bool,
    pub entries: Vec<SharedGameOption>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedGameOption {
    /// The `options.txt` key, e.g. `fov` or `key_key.attack`.
    pub key: String,
    /// The raw value, already in the game's own encoding (FOV as -1.0..1.0 and
    /// so on). Encoding lives in the frontend catalogue, not here.
    pub value: String,
    /// Only overwrite a key the instance's `options.txt` already has. Keeps the
    /// profile from introducing keys a given game version doesn't understand.
    #[serde(default)]
    pub only_if_present: bool,
    /// Inclusive lower bound on the Minecraft release this entry applies to,
    /// e.g. `"1.13"` for the post-flattening key binding format.
    #[serde(default)]
    pub min_version: Option<String>,
    /// Inclusive upper bound, for options a later version removed.
    #[serde(default)]
    pub max_version: Option<String>,
}

/// Parses a Minecraft *release* version (`1.20`, `1.20.1`) into comparable
/// parts. Snapshots and other non-release names deliberately return `None`.
fn parse_release_version(version: &str) -> Option<Vec<u32>> {
    let parts = version
        .split('.')
        .map(|part| part.parse::<u32>().ok())
        .collect::<Option<Vec<_>>>()?;

    if parts.is_empty() { None } else { Some(parts) }
}

impl SharedGameOption {
    /// Whether this entry should be written to an instance on `game_version`.
    ///
    /// Versions that aren't plain releases (snapshots, April Fools' builds) are
    /// treated as newer than every bound. In practice the snapshots people run
    /// in a launcher are current ones, so being permissive there beats dropping
    /// every version-gated option on them.
    pub fn applies_to(&self, game_version: &str) -> bool {
        if self.min_version.is_none() && self.max_version.is_none() {
            return true;
        }

        let Some(instance) = parse_release_version(game_version) else {
            // Unparseable: satisfies lower bounds, fails upper ones.
            return self.max_version.is_none();
        };

        if let Some(min) =
            self.min_version.as_deref().and_then(parse_release_version)
            && instance < min
        {
            return false;
        }

        if let Some(max) =
            self.max_version.as_deref().and_then(parse_release_version)
            && instance > max
        {
            return false;
        }

        true
    }
}

impl SharedGameOptions {
    /// Entries that apply to an instance on `game_version`, in stored order.
    pub fn applicable_to(
        &self,
        game_version: &str,
    ) -> impl Iterator<Item = &SharedGameOption> {
        let enabled = self.enabled;
        self.entries
            .iter()
            .filter(move |entry| enabled && entry.applies_to(game_version))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    PagePath,
    ProjectBackground,
    WorldsInHome,
    ServerRamAsBytesAlwaysOn,
    AlwaysShowAppControls,
    SkipUnknownPackWarning,
    PrideFundraiser,
    ServersInApp,
    ServerProjectQa,
    I18nDebug,
    ShowInstancePlayTime,
    SkipNonEssentialWarnings,
    AdvancedFiltersCollapsed,
    AlwaysShowCopyDetails,
    HideInstalledModpacks,
    FriendsActiveCollapsed,
    FriendsOnlineCollapsed,
    FriendsOfflineCollapsed,
    FriendsPendingCollapsed,
    DismissedPhotosensitivityFilterWarning,
}

impl Settings {
    const CURRENT_VERSION: usize = 3;

    pub async fn get(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<Self> {
        // proxy_url and shared_game_options are read with runtime-checked
        // queries so the sqlx offline cache (.sqlx) does not need regenerating
        // for these fork-only columns.
        let proxy_url: Option<String> =
            sqlx::query_scalar("SELECT proxy_url FROM settings")
                .fetch_one(exec)
                .await?;

        let shared_game_options: Option<String> =
            sqlx::query_scalar("SELECT shared_game_options FROM settings")
                .fetch_one(exec)
                .await?;

        let res = sqlx::query!(
            "
            SELECT
                max_concurrent_writes, max_concurrent_downloads,
                theme, locale, default_page, collapsed_navigation, hide_nametag_skins_page, advanced_rendering, native_decorations,
                discord_rpc, developer_mode, telemetry, personalized_ads,
                json(extra_launch_args) extra_launch_args, json(custom_env_vars) custom_env_vars,
                mc_memory_max, mc_force_fullscreen, mc_game_resolution_x, mc_game_resolution_y, hide_on_process_start,
                hook_pre_launch, hook_wrapper, hook_post_exit,
                custom_dir, prev_custom_dir, migrated, json(feature_flags) feature_flags, toggle_sidebar,
                skipped_update, pending_update_toast_for_version, auto_download_updates,
                version
            FROM settings
            "
        )
            .fetch_one(exec)
            .await?;

        Ok(Self {
            max_concurrent_downloads: res.max_concurrent_downloads as usize,
            max_concurrent_writes: res.max_concurrent_writes as usize,
            theme: Theme::from_string(&res.theme),
            locale: res.locale,
            default_page: DefaultPage::from_string(&res.default_page),
            collapsed_navigation: res.collapsed_navigation == 1,
            hide_nametag_skins_page: res.hide_nametag_skins_page == 1,
            advanced_rendering: res.advanced_rendering == 1,
            native_decorations: res.native_decorations == 1,
            toggle_sidebar: res.toggle_sidebar == 1,
            telemetry: res.telemetry == 1,
            discord_rpc: res.discord_rpc == 1,
            developer_mode: res.developer_mode == 1,
            personalized_ads: res.personalized_ads == 1,
            extra_launch_args: res
                .extra_launch_args
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            custom_env_vars: res
                .custom_env_vars
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            memory: MemorySettings {
                maximum: res.mc_memory_max as u32,
            },
            force_fullscreen: res.mc_force_fullscreen == 1,
            game_resolution: WindowSize(
                res.mc_game_resolution_x as u16,
                res.mc_game_resolution_y as u16,
            ),
            hide_on_process_start: res.hide_on_process_start == 1,
            hooks: Hooks {
                pre_launch: res.hook_pre_launch,
                wrapper: res.hook_wrapper,
                post_exit: res.hook_post_exit,
            },
            custom_dir: res.custom_dir,
            prev_custom_dir: res.prev_custom_dir,
            migrated: res.migrated == 1,
            feature_flags: res
                .feature_flags
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            skipped_update: res.skipped_update,
            pending_update_toast_for_version: res
                .pending_update_toast_for_version,
            auto_download_updates: res.auto_download_updates.map(|x| x == 1),
            proxy_url,
            shared_game_options: shared_game_options
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_default(),
            version: res.version as usize,
        })
    }

    pub async fn update(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    ) -> crate::Result<()> {
        let max_concurrent_writes = self.max_concurrent_writes as i32;
        let max_concurrent_downloads = self.max_concurrent_downloads as i32;
        let theme = self.theme.as_str();
        let default_page = self.default_page.as_str();
        let extra_launch_args = serde_json::to_string(&self.extra_launch_args)?;
        let custom_env_vars = serde_json::to_string(&self.custom_env_vars)?;
        let feature_flags = serde_json::to_string(&self.feature_flags)?;
        let version = self.version as i64;

        sqlx::query!(
            "
            UPDATE settings
            SET
                max_concurrent_writes = $1,
                max_concurrent_downloads = $2,

                theme = $3,
                locale = $4,
                default_page = $5,
                collapsed_navigation = $6,
                advanced_rendering = $7,
                native_decorations = $8,

                discord_rpc = $9,
                developer_mode = $10,
                telemetry = $11,
                personalized_ads = $12,

                extra_launch_args = jsonb($13),
                custom_env_vars = jsonb($14),
                mc_memory_max = $15,
                mc_force_fullscreen = $16,
                mc_game_resolution_x = $17,
                mc_game_resolution_y = $18,
                hide_on_process_start = $19,

                hook_pre_launch = $20,
                hook_wrapper = $21,
                hook_post_exit = $22,

                custom_dir = $23,
                prev_custom_dir = $24,
                migrated = $25,

                toggle_sidebar = $26,
                feature_flags = $27,
                hide_nametag_skins_page = $28,

                skipped_update = $29,
                pending_update_toast_for_version = $30,
                auto_download_updates = $31,

                version = $32
            ",
            max_concurrent_writes,
            max_concurrent_downloads,
            theme,
            self.locale,
            default_page,
            self.collapsed_navigation,
            self.advanced_rendering,
            self.native_decorations,
            self.discord_rpc,
            self.developer_mode,
            self.telemetry,
            self.personalized_ads,
            extra_launch_args,
            custom_env_vars,
            self.memory.maximum,
            self.force_fullscreen,
            self.game_resolution.0,
            self.game_resolution.1,
            self.hide_on_process_start,
            self.hooks.pre_launch,
            self.hooks.wrapper,
            self.hooks.post_exit,
            self.custom_dir,
            self.prev_custom_dir,
            self.migrated,
            self.toggle_sidebar,
            feature_flags,
            self.hide_nametag_skins_page,
            self.skipped_update,
            self.pending_update_toast_for_version,
            self.auto_download_updates,
            version,
        )
        .execute(exec)
        .await?;

        // Runtime-checked for the same reason as in `get` — fork-only columns.
        sqlx::query("UPDATE settings SET proxy_url = $1")
            .bind(&self.proxy_url)
            .execute(exec)
            .await?;

        let shared_game_options =
            serde_json::to_string(&self.shared_game_options)?;
        sqlx::query("UPDATE settings SET shared_game_options = $1")
            .bind(&shared_game_options)
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn migrate(exec: &Pool<Sqlite>) -> crate::Result<()> {
        let mut settings = Self::get(exec).await?;

        if settings.version < Settings::CURRENT_VERSION {
            tracing::info!(
                "Migrating settings version {} to {:?}",
                settings.version,
                Settings::CURRENT_VERSION
            );
        }
        while settings.version < Settings::CURRENT_VERSION {
            if let Err(err) = settings.perform_migration() {
                tracing::error!(
                    "Failed to migrate settings from version {}: {}",
                    settings.version,
                    err
                );
                return Err(err);
            }
        }

        settings.update(exec).await?;

        Ok(())
    }

    pub fn perform_migration(&mut self) -> crate::Result<()> {
        match self.version {
            1 => {
                let quoter = shlex::Quoter::new().allow_nul(true);

                // Previously split by spaces
                if let Some(pre_launch) = self.hooks.pre_launch.as_ref() {
                    self.hooks.pre_launch =
                        Some(quoter.join(pre_launch.split(' ')).unwrap())
                }

                // Previously treated as complete path to command
                if let Some(wrapper) = self.hooks.wrapper.as_ref() {
                    self.hooks.wrapper =
                        Some(quoter.quote(wrapper).unwrap().to_string())
                }

                // Previously split by spaces
                if let Some(post_exit) = self.hooks.post_exit.as_ref() {
                    self.hooks.post_exit =
                        Some(quoter.join(post_exit.split(' ')).unwrap())
                }

                self.version = 2;
            }
            2 => {
                // Update old default memory setting from 2GB to 4GB (depending on system memory)
                const LEGACY_DEFAULT_MEMORY_MB: u32 = 2048;
                if self.memory.maximum == LEGACY_DEFAULT_MEMORY_MB {
                    self.memory.maximum =
                        crate::api::jre::default_memory_max_mb();
                }

                self.version = 3;
            }
            version => {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Invalid settings version: {version}"
                ))
                .into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod shared_game_options_tests {
    use super::*;

    fn option(min: Option<&str>, max: Option<&str>) -> SharedGameOption {
        SharedGameOption {
            key: "fov".to_string(),
            value: "0.5".to_string(),
            only_if_present: false,
            min_version: min.map(str::to_string),
            max_version: max.map(str::to_string),
        }
    }

    #[test]
    fn unbounded_entries_apply_everywhere() {
        let entry = option(None, None);
        assert!(entry.applies_to("1.7.10"));
        assert!(entry.applies_to("1.21.4"));
        assert!(entry.applies_to("24w14a"));
    }

    #[test]
    fn minimum_version_is_inclusive() {
        let entry = option(Some("1.13"), None);
        assert!(!entry.applies_to("1.12.2"));
        assert!(entry.applies_to("1.13"));
        assert!(entry.applies_to("1.13.1"));
        assert!(entry.applies_to("1.21"));
    }

    #[test]
    fn maximum_version_is_inclusive() {
        let entry = option(None, Some("1.16.5"));
        assert!(entry.applies_to("1.8.9"));
        assert!(entry.applies_to("1.16.5"));
        assert!(!entry.applies_to("1.17"));
    }

    #[test]
    fn minor_versions_compare_numerically_not_lexically() {
        // "1.9" must not read as newer than "1.20" the way string ordering would.
        let entry = option(Some("1.20"), None);
        assert!(!entry.applies_to("1.9.4"));
        assert!(entry.applies_to("1.20.1"));
    }

    #[test]
    fn shorter_versions_sort_before_their_patches() {
        let entry = option(Some("1.20.1"), None);
        assert!(!entry.applies_to("1.20"));
        assert!(entry.applies_to("1.20.1"));
        assert!(entry.applies_to("1.20.2"));
    }

    #[test]
    fn snapshots_satisfy_lower_bounds_but_not_upper_ones() {
        assert!(option(Some("1.13"), None).applies_to("24w14a"));
        assert!(!option(None, Some("1.16.5")).applies_to("24w14a"));
    }

    #[test]
    fn disabled_profile_yields_nothing() {
        let profile = SharedGameOptions {
            enabled: false,
            entries: vec![option(None, None)],
        };
        assert_eq!(profile.applicable_to("1.20.1").count(), 0);
    }

    #[test]
    fn enabled_profile_filters_by_version() {
        let profile = SharedGameOptions {
            enabled: true,
            entries: vec![option(None, None), option(Some("1.13"), None)],
        };
        assert_eq!(profile.applicable_to("1.20.1").count(), 2);
        assert_eq!(profile.applicable_to("1.12.2").count(), 1);
    }
}

/// Theseus theme
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    Oled,
    Retro,
    System,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Oled => "oled",
            Theme::Retro => "retro",
            Theme::System => "system",
        }
    }

    pub fn from_string(string: &str) -> Theme {
        match string {
            "dark" => Theme::Dark,
            "light" => Theme::Light,
            "oled" => Theme::Oled,
            "retro" => Theme::Retro,
            "system" => Theme::System,
            _ => Theme::Dark,
        }
    }
}

/// Minecraft memory settings
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MemorySettings {
    pub maximum: u32,
}

/// Game window size
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WindowSize(pub u16, pub u16);

/// Game initialization hooks
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde_with::serde_as]
pub struct Hooks {
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub pre_launch: Option<String>,
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub wrapper: Option<String>,
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub post_exit: Option<String>,
}

/// Opening window to start with
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DefaultPage {
    Home,
    Library,
}

impl DefaultPage {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefaultPage::Home => "home",
            DefaultPage::Library => "library",
        }
    }

    pub fn from_string(string: &str) -> Self {
        match string {
            "home" => Self::Home,
            "library" => Self::Library,
            _ => Self::Home,
        }
    }
}
