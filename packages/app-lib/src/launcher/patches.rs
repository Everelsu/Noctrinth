//! Per-instance version patches in the MultiMC/Prism component format.
//!
//! Prism Launcher builds a version out of ordered *components*, each described
//! by a JSON file in the instance's `patches` folder. Mods that make legacy
//! Minecraft run on modern Java — lwjgl3ify being the notable one — ship
//! exactly those files: one patch swaps the LWJGL 2 component for LWJGL 3,
//! another adds the `--add-opens` wall as `+jvmArgs`, another replaces the
//! main class with a bootstrap that installs a custom system class loader.
//!
//! Patches are applied on top of the already-merged vanilla + loader
//! [`VersionInfo`]. They cannot be baked into it because
//! [`download_version_info`](super::download::download_version_info) caches its
//! result globally per version ID, while patches belong to a single instance.

use daedalus::minecraft::{Argument, ArgumentType, Library, Os, VersionInfo};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Folder inside an instance that holds the patch files.
pub const PATCHES_DIR: &str = "patches";

/// Folder inside an instance that holds jars referenced with `MMC-hint: local`.
pub const LOCAL_LIBRARIES_DIR: &str = "libraries";

/// The classpath slot the launcher's own libraries occupy.
///
/// Prism assembles the classpath by component order, and gives the game and its
/// mod loader the low slots — `net.minecraft` and `org.lwjgl3` sit below zero,
/// `net.minecraftforge` at 5. Everything Daedalus builds lands in that range, so
/// a patch ordered below it means to be loaded *before* the loader and a patch
/// ordered above it means to come last. lwjgl3ify relies on exactly this: its
/// early-classpath component is ordered 3 so its patched copies of Forge
/// classes win over the ones in the Forge jar, while its launch-arguments
/// component is ordered 100.
const LAUNCHER_LIBRARY_ORDER: i32 = 10;

/// A reference to another component, used by `conflicts` and `requires`.
#[derive(Deserialize, Debug)]
pub struct ComponentRef {
    /// The unique ID of the referenced component.
    pub uid: String,
}

/// A single component file from an instance's `patches` folder.
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionPatch {
    /// The unique ID of this component, e.g. `org.lwjgl3`.
    #[serde(default)]
    pub uid: Option<String>,
    /// Human-readable name shown by Prism. Only used for logging.
    #[serde(default)]
    pub name: Option<String>,
    /// The version of the component this patch describes.
    #[serde(default)]
    pub version: Option<String>,
    /// Where this component sits in the classpath. Lower comes first.
    #[serde(default)]
    pub order: Option<i32>,
    /// Replaces the main class the game is launched with.
    #[serde(default)]
    pub main_class: Option<String>,
    /// Replaces the legacy game argument string.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    /// Java major versions this component declares support for. Overrides the
    /// `javaVersion` Mojang's manifest asks for.
    #[serde(default)]
    pub compatible_java_majors: Option<Vec<u32>>,
    /// The libraries this component provides.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// Extra libraries appended on top of `libraries`.
    #[serde(default, rename = "+libraries")]
    pub added_libraries: Vec<Library>,
    /// Extra JVM arguments.
    #[serde(default, rename = "+jvmArgs")]
    pub jvm_args: Vec<String>,
    /// Extra game arguments.
    #[serde(default, rename = "+gameArgs")]
    pub game_args: Vec<String>,
    /// LaunchWrapper tweak classes to register.
    #[serde(default, rename = "+tweakers")]
    pub tweakers: Vec<String>,
    /// Components this one replaces. Their libraries are dropped.
    #[serde(default)]
    pub conflicts: Vec<ComponentRef>,
}

/// What applying a set of patches changed outside of [`VersionInfo`] itself.
#[derive(Debug, Default)]
pub struct AppliedPatches {
    /// JVM arguments contributed by the patches, in component order.
    pub jvm_args: Vec<String>,
    /// Java major versions the patched instance declares support for.
    pub compatible_java_majors: Option<Vec<u32>>,
}

impl AppliedPatches {
    /// The Java major version to prefer for this instance, if the patches
    /// override Mojang's choice. The lowest declared version wins, matching how
    /// Prism picks a runtime out of `compatibleJavaMajors`.
    pub fn preferred_java_major(&self) -> Option<u32> {
        self.compatible_java_majors
            .as_ref()
            .and_then(|majors| majors.iter().copied().min())
    }
}

/// Maven group prefixes owned by the components the launcher knows about.
///
/// Patch files declare conflicts by component ID, but the merged
/// [`VersionInfo`] has no notion of components, so conflicts are resolved
/// against the maven coordinates those components are known to provide.
fn conflicting_library_groups(uid: &str) -> &'static [&'static str] {
    match uid {
        // LWJGL 2, plus the input libraries Prism bundles into the component.
        "org.lwjgl" => {
            &["org.lwjgl.lwjgl", "net.java.jinput", "net.java.jutils"]
        }
        "org.lwjgl3" => &["org.lwjgl"],
        _ => &[],
    }
}

/// The `group:artifact[:classifier]` identity of a maven coordinate, ignoring
/// the version so that a patch can upgrade a library in place.
fn library_key(name: &str) -> String {
    let name = name.split_once('@').map_or(name, |(name, _)| name);
    let mut parts = name.split(':');
    let group = parts.next().unwrap_or_default();
    let artifact = parts.next().unwrap_or_default();
    let _version = parts.next();

    match parts.next() {
        Some(classifier) => format!("{group}:{artifact}:{classifier}"),
        None => format!("{group}:{artifact}"),
    }
}

/// The version part of a maven coordinate.
fn library_version(name: &str) -> Option<&str> {
    name.split_once('@')
        .map_or(name, |(coordinate, _)| coordinate)
        .split(':')
        .nth(2)
}

/// Orders two maven versions by comparing their parts, numerically where both
/// sides are numbers.
fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let separators = |c: char| c == '.' || c == '-' || c == '_';
    let mut left = left.split(separators);
    let mut right = right.split(separators);

    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left), Some(right)) => {
                let ordering = match (left.parse::<u64>(), right.parse::<u64>())
                {
                    (Ok(left), Ok(right)) => left.cmp(&right),
                    _ => left.cmp(right),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

/// Whether taking the patch's copy of a library would downgrade it.
///
/// MultiMC resolves two components claiming the same artifact by keeping the
/// highest version, which is how Forge's Guava 17 survives alongside the
/// Minecraft component's Guava 15. Replacing blindly would hand the game the
/// older jar and lose every class added since — `Runnables`, for one, which the
/// mixin bootstraps call into.
fn is_downgrade(candidate: &str, existing: &str) -> bool {
    match (library_version(candidate), library_version(existing)) {
        (Some(candidate), Some(existing)) => {
            compare_versions(candidate, existing) == std::cmp::Ordering::Less
        }
        _ => false,
    }
}

/// The LWJGL 3 native classifier that matches the running platform.
///
/// Components in the MultiMC format carry one artifact per platform, named
/// `...-natives-<platform>`, and gate them behind a rule that only names the
/// operating system: all three Windows artifacts are allowed by
/// `{"os": {"name": "windows"}}` alike. The architecture lives in the artifact
/// name, so that is where it has to be read from.
fn host_native_classifier(java_arch: &str) -> Option<&'static str> {
    Some(match Os::native_arch(java_arch) {
        Os::Windows if java_arch == "x86" => "windows-x86",
        Os::Windows => "windows",
        Os::WindowsArm64 => "windows-arm64",
        Os::Linux => "linux",
        Os::LinuxArm64 => "linux-arm64",
        Os::LinuxArm32 => "linux-arm32",
        Os::Osx => "macos",
        Os::OsxArm64 => "macos-arm64",
        Os::Unknown => return None,
    })
}

/// Whether a patch library is a platform-specific native for a platform other
/// than the one being launched on. Non-native artifacts are never rejected.
fn is_foreign_native(name: &str, java_arch: &str) -> bool {
    let mut parts = name.split(':');
    let _group = parts.next();
    let Some(artifact) = parts.next() else {
        return false;
    };
    let Some((_, platform)) = artifact.split_once("-natives-") else {
        return false;
    };

    host_native_classifier(java_arch) != Some(platform)
}

/// The bare file name a maven coordinate resolves to.
///
/// Libraries hinted as `local` are looked up by file name directly inside the
/// instance's `libraries` folder — MultiMC stores them flat there rather than
/// in a maven tree, and the archives shipped by mods follow suit.
pub fn maven_file_name(name: &str) -> String {
    let (coordinate, extension) = name
        .split_once('@')
        .map_or((name, "jar"), |(coordinate, extension)| {
            (coordinate, extension)
        });

    let mut parts = coordinate.split(':');
    let _group = parts.next();
    let artifact = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();

    match parts.next() {
        Some(classifier) => {
            format!("{artifact}-{version}-{classifier}.{extension}")
        }
        None => format!("{artifact}-{version}.{extension}"),
    }
}

/// Whether a library's maven group is the given group or nested under it.
fn library_in_group(name: &str, group: &str) -> bool {
    let library_group = name.split(':').next().unwrap_or_default();
    library_group == group
        || library_group
            .strip_prefix(group)
            .is_some_and(|rest| rest.starts_with('.'))
}

/// Reads every patch in an instance's `patches` folder, ordered the way Prism
/// would apply them: by `order`, then by component ID for a stable tie-break.
pub fn load_instance_patches(
    instance_path: &Path,
) -> crate::Result<Vec<VersionPatch>> {
    let dir = instance_path.join(PATCHES_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(crate::ErrorKind::InputError(format!(
                "Could not read version patches in {}: {error}",
                dir.display()
            ))
            .into());
        }
    };

    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extension.eq_ignore_ascii_case("json")
                    })
        })
        .collect::<Vec<_>>();
    paths.sort();

    let mut patches = Vec::with_capacity(paths.len());
    for path in paths {
        let contents = std::fs::read(&path).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not read version patch {}: {error}",
                path.display()
            ))
        })?;
        let patch: VersionPatch =
            serde_json::from_slice(&contents).map_err(|error| {
                crate::ErrorKind::InputError(format!(
                    "Could not parse version patch {}: {error}",
                    path.display()
                ))
            })?;
        patches.push(patch);
    }

    patches.sort_by(|a, b| {
        a.order
            .unwrap_or(0)
            .cmp(&b.order.unwrap_or(0))
            .then_with(|| a.uid.cmp(&b.uid))
    });

    Ok(patches)
}

/// Applies ordered patches on top of a merged version, returning everything the
/// caller still has to act on.
pub fn apply_patches(
    version_info: &mut VersionInfo,
    patches: &[VersionPatch],
) -> AppliedPatches {
    let mut applied = AppliedPatches::default();

    if patches.is_empty() {
        return applied;
    }

    // A conflict rules a whole component out of the version, so it has to apply
    // to what the patches themselves bring in as much as to what is already
    // there. lwjgl3ify's Minecraft component ships the full vanilla library
    // list, jinput and jutils included, and those belong to the LWJGL 2
    // component its LWJGL 3 patch just declared a conflict with.
    let conflicting_groups = patches
        .iter()
        .flat_map(|patch| &patch.conflicts)
        .flat_map(|conflict| conflicting_library_groups(&conflict.uid))
        .copied()
        .collect::<Vec<_>>();
    let conflicts = |name: &str| {
        conflicting_groups
            .iter()
            .any(|group| library_in_group(name, group))
    };

    version_info
        .libraries
        .retain(|library| !conflicts(&library.name));

    let mut prepended: Vec<Library> = Vec::new();
    let mut game_args: Vec<String> = Vec::new();

    for patch in patches {
        let leading = patch.order.unwrap_or(0) < LAUNCHER_LIBRARY_ORDER;

        for library in patch.libraries.iter().chain(&patch.added_libraries) {
            // Dropped here rather than at download or classpath time so that
            // both see the same set: a native that reaches the classpath but
            // not the download makes the launch fail on a missing file.
            if is_foreign_native(&library.name, std::env::consts::ARCH)
                || conflicts(&library.name)
            {
                continue;
            }

            let mut library = library.clone();
            if library.is_local() {
                // The jar lives in the instance folder; nothing to fetch.
                library.downloadable = false;
            }

            let key = library_key(&library.name);
            let existing = version_info
                .libraries
                .iter_mut()
                .find(|existing| library_key(&existing.name) == key);

            match existing {
                Some(existing)
                    if is_downgrade(&library.name, &existing.name) =>
                {
                    tracing::debug!(
                        "Keeping {} over the patched {}",
                        existing.name,
                        library.name
                    );
                }
                Some(existing) => {
                    // A patch updates the artifact, it does not re-decide
                    // classpath membership: merging the loader manifest already
                    // switched off the vanilla copies of libraries the loader
                    // ships itself, and re-enabling them here would put both on
                    // the classpath.
                    library.include_in_classpath &=
                        existing.include_in_classpath;
                    *existing = library;
                }
                None if leading => prepended.push(library),
                None => version_info.libraries.push(library),
            }
        }

        if let Some(main_class) = &patch.main_class {
            version_info.main_class.clone_from(main_class);
        }

        if let Some(minecraft_arguments) = &patch.minecraft_arguments {
            version_info.minecraft_arguments =
                Some(minecraft_arguments.clone());
        }

        if let Some(majors) = &patch.compatible_java_majors {
            applied.compatible_java_majors = Some(majors.clone());
        }

        applied.jvm_args.extend(patch.jvm_args.iter().cloned());
        game_args.extend(patch.game_args.iter().cloned());
        for tweaker in &patch.tweakers {
            game_args.push("--tweakClass".to_string());
            game_args.push(tweaker.clone());
        }
    }

    if !prepended.is_empty() {
        prepended.append(&mut version_info.libraries);
        version_info.libraries = prepended;
    }

    if !game_args.is_empty() {
        append_game_arguments(version_info, game_args);
    }

    applied
}

/// Appends game arguments to whichever of the two argument formats the version
/// uses. Versions before 1.13 carry a single `minecraftArguments` string.
fn append_game_arguments(version_info: &mut VersionInfo, args: Vec<String>) {
    if let Some(minecraft_arguments) = &mut version_info.minecraft_arguments {
        for arg in &args {
            minecraft_arguments.push(' ');
            minecraft_arguments.push_str(arg);
        }
        return;
    }

    let arguments = version_info
        .arguments
        .get_or_insert_with(HashMap::new)
        .entry(ArgumentType::Game)
        .or_default();
    arguments.extend(args.into_iter().map(Argument::Normal));
}

/// The Minecraft version a patch set was written for, if it says so.
///
/// The `net.minecraft` component carries the game version it describes, which
/// is what makes it possible to tell a patch set that still belongs to this
/// instance from one left behind by an earlier version.
fn patched_game_version(patches: &[VersionPatch]) -> Option<&str> {
    patches
        .iter()
        .find(|patch| patch.uid.as_deref() == Some("net.minecraft"))
        .and_then(|patch| patch.version.as_deref())
}

/// Loads an instance's patches and applies them to a merged version.
///
/// Patches are tied to one Minecraft version. Changing an instance's version
/// leaves the old ones on disk, and applying those would graft the previous
/// version's libraries and main class onto the new one, so a patch set that no
/// longer matches is ignored instead.
pub fn patch_version_info(
    instance_path: &Path,
    version_info: &mut VersionInfo,
    game_version: &str,
) -> crate::Result<AppliedPatches> {
    let patches = load_instance_patches(instance_path)?;
    if patches.is_empty() {
        return Ok(AppliedPatches::default());
    }

    if let Some(patched_version) = patched_game_version(&patches)
        && patched_version != game_version
    {
        tracing::warn!(
            "Ignoring version patches in {}: they are for Minecraft {patched_version}, but the instance runs {game_version}",
            instance_path.join(PATCHES_DIR).display(),
        );
        return Ok(AppliedPatches::default());
    }

    tracing::info!(
        "Applying version patches from {}: {}",
        instance_path.join(PATCHES_DIR).display(),
        patches
            .iter()
            .map(|patch| patch
                .name
                .as_deref()
                .or(patch.uid.as_deref())
                .unwrap_or("unnamed"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(apply_patches(version_info, &patches))
}

/// The folder an instance's `MMC-hint: local` libraries are resolved against.
pub fn local_libraries_dir(instance_path: &Path) -> PathBuf {
    instance_path.join(LOCAL_LIBRARIES_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str) -> Library {
        Library {
            downloads: None,
            extract: None,
            name: name.to_string(),
            url: None,
            natives: None,
            rules: None,
            checksums: None,
            include_in_classpath: true,
            downloadable: true,
            mmc_hint: None,
        }
    }

    fn patch(json: &str) -> VersionPatch {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn patches_load_in_component_order_not_file_order() {
        let instance = tempfile::tempdir().unwrap();
        let dir = instance.path().join(PATCHES_DIR);
        std::fs::create_dir_all(&dir).unwrap();

        // Alphabetically these land in the wrong order on purpose.
        std::fs::write(
            dir.join("me.eigenraven.lwjgl3ify.launchargs.json"),
            r#"{ "uid": "me.eigenraven.lwjgl3ify.launchargs", "order": 100 }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("net.minecraft.json"),
            r#"{ "uid": "net.minecraft" }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("org.lwjgl3.json"),
            r#"{ "uid": "org.lwjgl3", "order": -1 }"#,
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let patches = load_instance_patches(instance.path()).unwrap();
        let uids: Vec<_> = patches
            .iter()
            .map(|patch| patch.uid.as_deref().unwrap())
            .collect();

        assert_eq!(
            uids,
            vec![
                "org.lwjgl3",
                "net.minecraft",
                "me.eigenraven.lwjgl3ify.launchargs"
            ]
        );
    }

    #[test]
    fn rules_for_unsupported_platforms_do_not_fail_the_patch() {
        // Verbatim shape from lwjgl3ify's shipped org.lwjgl3.json, which lists
        // natives for platforms the launcher has no `Os` variant for.
        let patch = patch(
            r#"{
                "uid": "org.lwjgl3", "order": -1,
                "conflicts": [{ "uid": "org.lwjgl" }],
                "libraries": [
                    {
                        "name": "org.lwjgl:lwjgl-freetype-natives-freebsd:3.4.2",
                        "downloads": { "artifact": {
                            "sha1": "c76950cce5113badc2e2efc4239cb53cf3aeb10b",
                            "size": 1217460,
                            "url": "https://repo1.maven.org/maven2/org/lwjgl/lwjgl-freetype/3.4.2/lwjgl-freetype-3.4.2-natives-freebsd.jar"
                        } },
                        "rules": [{ "action": "allow", "os": { "name": "freebsd" } }]
                    },
                    { "name": "org.lwjgl:lwjgl-freetype:3.4.2" }
                ]
            }"#,
        );

        assert_eq!(patch.libraries.len(), 2);
        let freebsd_rules = patch.libraries[0].rules.as_ref().unwrap();
        assert!(matches!(
            freebsd_rules[0].os.as_ref().unwrap().name,
            Some(daedalus::minecraft::Os::Unknown)
        ));
    }

    #[test]
    fn patches_left_behind_by_an_earlier_version_are_ignored() {
        let instance = tempfile::tempdir().unwrap();
        let dir = instance.path().join(PATCHES_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("net.minecraft.json"),
            r#"{ "uid": "net.minecraft", "version": "1.7.10",
                 "compatibleJavaMajors": [17, 21] }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("me.eigenraven.lwjgl3ify.launchargs.json"),
            r#"{ "uid": "me.eigenraven.lwjgl3ify.launchargs", "order": 100,
                 "mainClass": "com.gtnewhorizons.retrofuturabootstrap.Main" }"#,
        )
        .unwrap();

        // Same version: the patches still describe this instance.
        let mut version_info = version_with(vec![]);
        let applied =
            patch_version_info(instance.path(), &mut version_info, "1.7.10")
                .unwrap();
        assert_eq!(applied.preferred_java_major(), Some(17));
        assert_eq!(
            version_info.main_class,
            "com.gtnewhorizons.retrofuturabootstrap.Main"
        );

        // Changed version: applying them would graft 1.7.10 onto 1.20.1.
        let mut version_info = version_with(vec![]);
        let applied =
            patch_version_info(instance.path(), &mut version_info, "1.20.1")
                .unwrap();
        assert_eq!(applied.preferred_java_major(), None);
        assert_eq!(version_info.main_class, "net.minecraft.client.main.Main");
    }

    #[test]
    fn an_instance_without_patches_loads_nothing() {
        let instance = tempfile::tempdir().unwrap();
        assert!(load_instance_patches(instance.path()).unwrap().is_empty());
    }

    #[test]
    fn a_malformed_patch_fails_the_launch_instead_of_being_ignored() {
        let instance = tempfile::tempdir().unwrap();
        let dir = instance.path().join(PATCHES_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("org.lwjgl3.json"), "{ not json").unwrap();

        let error = load_instance_patches(instance.path()).unwrap_err();
        assert!(error.to_string().contains("org.lwjgl3.json"));
    }

    #[test]
    fn library_key_ignores_version_but_keeps_classifier() {
        assert_eq!(
            library_key("org.lwjgl.lwjgl:lwjgl:2.9.1"),
            "org.lwjgl.lwjgl:lwjgl"
        );
        assert_eq!(
            library_key("org.lwjgl.lwjgl:lwjgl-platform:2.9.1:natives-windows"),
            "org.lwjgl.lwjgl:lwjgl-platform:natives-windows"
        );
        assert_eq!(
            library_key("net.minecraftforge:forge:1.7.10@jar"),
            "net.minecraftforge:forge"
        );
    }

    #[test]
    fn native_artifacts_are_matched_against_the_running_platform() {
        // All three Windows artifacts carry the same os-only rule, so the
        // architecture has to come out of the artifact name.
        assert!(!is_foreign_native(
            "org.lwjgl:lwjgl-freetype-natives-windows:3.4.2",
            "x86_64"
        ));
        assert!(is_foreign_native(
            "org.lwjgl:lwjgl-freetype-natives-windows-x86:3.4.2",
            "x86_64"
        ));
        assert!(is_foreign_native(
            "org.lwjgl:lwjgl-freetype-natives-windows-arm64:3.4.2",
            "x86_64"
        ));
        // Artifacts that are not platform natives are never rejected.
        assert!(!is_foreign_native(
            "org.lwjgl:lwjgl-freetype:3.4.2",
            "x86_64"
        ));
        assert!(!is_foreign_native("com.mojang:netty:1.8.8", "x86_64"));
    }

    #[test]
    fn early_classpath_components_load_before_the_loader() {
        // lwjgl3ify's early-classpath jar carries patched copies of Forge
        // classes; landing after the Forge jar means the unpatched ones win and
        // the game dies on a class Java no longer has.
        let mut version_info = version_with(vec![library(
            "net.minecraftforge:forge:1.7.10:universal",
        )]);

        let patches = vec![
            patch(
                r#"{ "uid": "org.lwjgl3", "order": -1,
                     "libraries": [{ "name": "org.lwjgl:lwjgl:3.4.2" }] }"#,
            ),
            patch(
                r#"{ "uid": "me.eigenraven.lwjgl3ify.forgepatches", "order": 3,
                     "libraries": [{ "name": "com.github.GTNewHorizons:lwjgl3ify:3.0.31:forgePatches",
                                     "MMC-hint": "local" }] }"#,
            ),
            patch(
                r#"{ "uid": "me.eigenraven.lwjgl3ify.launchargs", "order": 100,
                     "libraries": [{ "name": "com.example:trailing:1.0" }] }"#,
            ),
        ];
        apply_patches(&mut version_info, &patches);

        let names: Vec<_> = version_info
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "org.lwjgl:lwjgl:3.4.2",
                "com.github.GTNewHorizons:lwjgl3ify:3.0.31:forgePatches",
                "net.minecraftforge:forge:1.7.10:universal",
                "com.example:trailing:1.0",
            ]
        );
    }

    #[test]
    fn natives_for_other_platforms_never_reach_the_version() {
        let mut version_info = version_with(vec![]);
        let patches = vec![patch(
            r#"{ "uid": "org.lwjgl3", "order": -1, "libraries": [
                { "name": "org.lwjgl:lwjgl-freetype-natives-freebsd:3.4.2" },
                { "name": "org.lwjgl:lwjgl-freetype:3.4.2" }
            ] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        let names: Vec<_> = version_info
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(names, vec!["org.lwjgl:lwjgl-freetype:3.4.2"]);
    }

    #[test]
    fn local_libraries_resolve_to_a_flat_file_name() {
        // The name lwjgl3ify's archive actually ships the jar under.
        assert_eq!(
            maven_file_name(
                "com.github.GTNewHorizons:lwjgl3ify:3.0.31:forgePatches"
            ),
            "lwjgl3ify-3.0.31-forgePatches.jar"
        );
        assert_eq!(maven_file_name("org.lwjgl:lwjgl:3.4.2"), "lwjgl-3.4.2.jar");
        assert_eq!(
            maven_file_name("net.minecraftforge:forge:1.7.10@zip"),
            "forge-1.7.10.zip"
        );
    }

    #[test]
    fn library_in_group_matches_nested_groups_only_on_boundaries() {
        assert!(library_in_group("org.lwjgl.lwjgl:lwjgl:2.9.1", "org.lwjgl"));
        assert!(library_in_group("org.lwjgl:lwjgl:3.3.3", "org.lwjgl"));
        assert!(!library_in_group("org.lwjgl3ify:thing:1.0", "org.lwjgl"));
    }

    #[test]
    fn conflicting_component_libraries_are_dropped() {
        let mut version_info = version_with(vec![
            library("org.lwjgl.lwjgl:lwjgl:2.9.1"),
            library("org.lwjgl.lwjgl:lwjgl-platform:2.9.1"),
            library("net.java.jinput:jinput:2.0.5"),
            library("net.java.jinput:jinput-platform:2.0.5"),
            library("com.mojang:netty:1.8.8"),
        ]);

        let patches = vec![patch(
            r#"{ "uid": "org.lwjgl3", "order": -1, "conflicts": [{ "uid": "org.lwjgl" }],
                 "libraries": [{ "name": "org.lwjgl:lwjgl:3.3.3" }] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        let names: Vec<_> = version_info
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["org.lwjgl:lwjgl:3.3.3", "com.mojang:netty:1.8.8"]
        );
    }

    #[test]
    fn a_patch_never_downgrades_a_library() {
        // lwjgl3ify's Minecraft component still lists the vanilla Guava 15,
        // while Forge needs the 17 it brings itself. Guava 16 added
        // `Runnables`, which the mixin bootstrap loads.
        let mut version_info =
            version_with(vec![library("com.google.guava:guava:17.0")]);

        let patches = vec![patch(
            r#"{ "uid": "net.minecraft", "order": -2,
                 "libraries": [{ "name": "com.google.guava:guava:15.0" }] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        assert_eq!(version_info.libraries.len(), 1);
        assert_eq!(
            version_info.libraries[0].name,
            "com.google.guava:guava:17.0"
        );
    }

    #[test]
    fn maven_versions_compare_part_by_part() {
        use std::cmp::Ordering;

        assert_eq!(compare_versions("17.0", "15.0"), Ordering::Greater);
        assert_eq!(compare_versions("2.9.1", "2.9.4"), Ordering::Less);
        assert_eq!(compare_versions("1.0", "1.0.1"), Ordering::Less);
        assert_eq!(compare_versions("3.4.2", "3.4.2"), Ordering::Equal);
        // Numeric parts sort numerically, not as text.
        assert_eq!(compare_versions("10.0", "9.0"), Ordering::Greater);
        assert!(!is_downgrade(
            "com.google.guava:guava:17.0",
            "com.google.guava:guava:15.0"
        ));
        assert!(is_downgrade(
            "com.google.guava:guava:15.0",
            "com.google.guava:guava:17.0"
        ));
    }

    #[test]
    fn a_conflicting_component_cannot_be_reintroduced_by_another_patch() {
        // lwjgl3ify ships the whole vanilla library list in its Minecraft
        // component, jinput and jutils included — the very LWJGL 2 pieces its
        // LWJGL 3 component declares a conflict with.
        let mut version_info = version_with(vec![
            library("net.java.jinput:jinput-platform:2.0.5"),
            library("com.mojang:netty:1.8.8"),
        ]);

        let patches = vec![
            patch(
                r#"{ "uid": "net.minecraft", "order": -2, "libraries": [
                    { "name": "net.java.jinput:jinput-platform:2.0.5" },
                    { "name": "net.java.jinput:jinput:2.0.5" },
                    { "name": "net.java.jutils:jutils:1.0.0" },
                    { "name": "com.mojang:netty:1.8.8" }
                ] }"#,
            ),
            patch(
                r#"{ "uid": "org.lwjgl3", "order": -1,
                     "conflicts": [{ "uid": "org.lwjgl" }],
                     "libraries": [{ "name": "org.lwjgl:lwjgl:3.4.2" }] }"#,
            ),
        ];
        apply_patches(&mut version_info, &patches);

        let names: Vec<_> = version_info
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["org.lwjgl:lwjgl:3.4.2", "com.mojang:netty:1.8.8"]
        );
    }

    #[test]
    fn same_coordinate_libraries_are_replaced_in_place() {
        let mut version_info = version_with(vec![
            library("com.mojang:netty:1.8.8"),
            library("net.minecraftforge:forge:1.7.10-10.13.4.1614:universal"),
        ]);

        let patches = vec![patch(
            r#"{ "uid": "net.minecraft",
                 "libraries": [{ "name": "com.mojang:netty:1.8.9" }] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        assert_eq!(version_info.libraries[0].name, "com.mojang:netty:1.8.9");
        assert_eq!(version_info.libraries.len(), 2);
    }

    #[test]
    fn replacing_a_library_keeps_it_off_the_classpath_if_the_loader_shipped_it()
    {
        let mut excluded = library("com.google.guava:guava:17.0");
        excluded.include_in_classpath = false;
        let mut version_info = version_with(vec![excluded]);

        let patches = vec![patch(
            r#"{ "uid": "net.minecraft",
                 "libraries": [{ "name": "com.google.guava:guava:17.0" }] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        assert!(!version_info.libraries[0].include_in_classpath);
    }

    #[test]
    fn main_class_jvm_args_and_tweakers_are_collected() {
        let mut version_info = version_with(vec![]);
        version_info.minecraft_arguments =
            Some("--username ${auth_player_name}".to_string());

        let patches = vec![
            patch(
                r#"{ "uid": "net.minecraftforge",
                     "+tweakers": ["cpw.mods.fml.common.launcher.FMLTweaker"] }"#,
            ),
            patch(
                r#"{ "uid": "me.eigenraven.lwjgl3ify.launchargs", "order": 100,
                     "mainClass": "com.gtnewhorizons.retrofuturabootstrap.Main",
                     "+jvmArgs": ["--add-opens", "java.base/java.net=ALL-UNNAMED"] }"#,
            ),
        ];
        let applied = apply_patches(&mut version_info, &patches);

        assert_eq!(
            version_info.main_class,
            "com.gtnewhorizons.retrofuturabootstrap.Main"
        );
        assert_eq!(
            version_info.minecraft_arguments.as_deref(),
            Some(
                "--username ${auth_player_name} --tweakClass cpw.mods.fml.common.launcher.FMLTweaker"
            )
        );
        assert_eq!(
            applied.jvm_args,
            vec!["--add-opens", "java.base/java.net=ALL-UNNAMED"]
        );
    }

    #[test]
    fn compatible_java_majors_pick_the_lowest_supported_runtime() {
        let mut version_info = version_with(vec![]);
        let patches = vec![patch(
            r#"{ "uid": "net.minecraft", "compatibleJavaMajors": [21, 17, 25] }"#,
        )];

        let applied = apply_patches(&mut version_info, &patches);
        assert_eq!(applied.preferred_java_major(), Some(17));
    }

    #[test]
    fn local_libraries_are_not_downloaded() {
        let mut version_info = version_with(vec![]);
        let patches = vec![patch(
            r#"{ "uid": "me.eigenraven.lwjgl3ify.forgepatches", "order": 3,
                 "libraries": [{ "name": "com.github.GTNewHorizons:lwjgl3ify:2.1.14:forgePatches",
                                 "MMC-hint": "local" }] }"#,
        )];
        apply_patches(&mut version_info, &patches);

        let library = &version_info.libraries[0];
        assert!(library.is_local());
        assert!(!library.downloadable);
    }

    fn version_with(libraries: Vec<Library>) -> VersionInfo {
        use daedalus::minecraft::{AssetIndex, VersionType};

        VersionInfo {
            arguments: None,
            asset_index: AssetIndex {
                id: "1.7.10".to_string(),
                sha1: String::new(),
                size: 0,
                total_size: 0,
                url: String::new(),
            },
            assets: "1.7.10".to_string(),
            downloads: HashMap::new(),
            id: "1.7.10".to_string(),
            java_version: None,
            libraries,
            logging: None,
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_arguments: None,
            minimum_launcher_version: 0,
            release_time: chrono::Utc::now(),
            time: chrono::Utc::now(),
            type_: VersionType::Release,
            data: None,
            processors: None,
        }
    }
}
