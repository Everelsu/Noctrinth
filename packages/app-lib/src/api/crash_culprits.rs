//! Which installed mod a crash points at.
//!
//! A stack trace names classes, a mixin failure names a config file, a Fabric
//! mixin handler carries its mod's id in its own method name, and a loader
//! error names a mod id. None of those is something a player can find in their
//! mod list. This maps each onto the jar in `mods/` that holds it, so the
//! answer is a file that can be switched off rather than a name to go and
//! search for.
//!
//! Reading a jar's directory is cheap — the central directory sits at the end
//! of the file and nothing is decompressed but the small metadata files — so a
//! pack of a few hundred mods is indexed in well under a second.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

/// One jar in `mods/`, and what it answers for.
#[derive(Debug, Default)]
pub struct ModJar {
    /// Relative to the instance, as the content API names it: `mods/x.jar`.
    pub file: String,
    /// What the mod calls itself, or the file name when it does not say.
    pub name: String,
    pub ids: Vec<String>,
    packages: HashSet<String>,
    mixin_configs: HashSet<String>,
}

#[derive(Debug, Default)]
pub struct ModIndex {
    pub jars: Vec<ModJar>,
    /// Jars that could not be opened as archives at all.
    pub broken: Vec<String>,
}

/// A mod the crash points at, and what gave it away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suspect {
    pub file: String,
    pub name: String,
    /// The class or handler in the trace that belongs to it.
    pub clue: String,
}

/// Packages that are the game, a loader or a library everything shares. A
/// frame in one of these is where the crash passed through, not who caused it —
/// and Fabric API does sit in `mods/`, so it has to be named here.
const FRAMEWORK_PREFIXES: &[&str] = &[
    "java.",
    "javax.",
    "jdk.",
    "sun.",
    "com.sun.",
    "net.minecraft.",
    "com.mojang.",
    "net.minecraftforge.",
    "net.neoforged.",
    "net.fabricmc.",
    "org.quiltmc.",
    "org.spongepowered.",
    "cpw.mods.",
    "org.lwjgl.",
    "io.netty.",
    "com.google.",
    "org.apache.",
    "it.unimi.",
    "org.objectweb.",
    "com.llamalad7.",
    "org.slf4j.",
    "kotlin.",
    "kotlinx.",
    "scala.",
    "org.jetbrains.",
    "com.electronwill.",
    "oshi.",
    "com.github.benmanes.",
];

fn is_framework(class: &str) -> bool {
    FRAMEWORK_PREFIXES
        .iter()
        .any(|prefix| class.starts_with(prefix))
}

impl ModIndex {
    /// Reads every enabled jar in `mods_dir`. Blocking; run it off the runtime.
    pub fn read(mods_dir: &Path) -> ModIndex {
        let mut index = ModIndex::default();
        let Ok(entries) = std::fs::read_dir(mods_dir) else {
            return index;
        };

        let mut files: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().is_some_and(|extension| extension == "jar")
            })
            .collect();
        files.sort();

        // A few hundred archives read one after another is seconds of disk
        // and inflating; spread over the cores it is a fraction of that.
        let workers = std::thread::available_parallelism()
            .map_or(4, |count| count.get())
            .min(8);
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut read: Vec<(usize, Option<ModJar>)> =
            std::thread::scope(|scope| {
                let handles: Vec<_> = (0..workers)
                    .map(|_| {
                        scope.spawn(|| {
                            let mut done = Vec::new();
                            loop {
                                let at = next.fetch_add(
                                    1,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                let Some(path) = files.get(at) else {
                                    return done;
                                };
                                done.push((at, read_jar(path)));
                            }
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|handle| handle.join().unwrap_or_default())
                    .collect()
            });
        read.sort_by_key(|(at, _)| *at);

        for (at, jar) in read {
            let file_name = files[at]
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let relative = format!("mods/{file_name}");

            match jar {
                Some(mut jar) => {
                    if jar.name.is_empty() {
                        jar.name =
                            file_name.trim_end_matches(".jar").to_string();
                    }
                    jar.ids.sort();
                    jar.ids.dedup();
                    jar.file = relative;
                    index.jars.push(jar);
                }
                None => index.broken.push(relative),
            }
        }

        index
    }

    pub fn by_id(&self, id: &str) -> Option<&ModJar> {
        self.jars
            .iter()
            .find(|jar| jar.ids.iter().any(|known| known == id))
    }

    pub fn by_mixin_config(&self, config: &str) -> Option<&ModJar> {
        let config = config.rsplit('/').next().unwrap_or(config);
        self.jars
            .iter()
            .find(|jar| jar.mixin_configs.contains(config))
    }

    /// The jar a class, or a member of one, comes from.
    pub fn by_symbol(&self, symbol: &str) -> Option<&ModJar> {
        let dotted = symbol.replace('/', ".");
        let parts: Vec<&str> = dotted.split('.').collect();

        // `pkg.Class` or `pkg.Class.member`: the package is everything before
        // the first capitalised part, or failing that, all but the last one or
        // two parts.
        let candidates =
            [parts.len().saturating_sub(1), parts.len().saturating_sub(2)];
        let capitalised = parts
            .iter()
            .position(|part| part.starts_with(char::is_uppercase));

        capitalised
            .into_iter()
            .chain(candidates)
            .filter(|&length| length > 0)
            .find_map(|length| {
                let package = parts[..length].join(".");
                self.jars.iter().find(|jar| jar.packages.contains(&package))
            })
    }

    /// Mod ids declared by more than one jar, with the jars that declare them.
    pub fn duplicate_ids(&self) -> Vec<(String, Vec<&ModJar>)> {
        let mut owners: HashMap<&str, Vec<&ModJar>> = HashMap::new();
        for jar in &self.jars {
            for id in &jar.ids {
                let jars = owners.entry(id).or_default();
                if !jars.iter().any(|known| known.file == jar.file) {
                    jars.push(jar);
                }
            }
        }

        let mut duplicates: Vec<_> = owners
            .into_iter()
            .filter(|(_, jars)| jars.len() > 1)
            .map(|(id, jars)| (id.to_string(), jars))
            .collect();
        duplicates.sort_by(|a, b| a.0.cmp(&b.0));
        duplicates
    }
}

fn read_jar(path: &Path) -> Option<ModJar> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut jar = ModJar::default();
    let mut metadata = Vec::new();
    let mut last_package = String::new();

    for index in 0..archive.len() {
        let Some(name) = archive.name_for_index(index) else {
            continue;
        };

        if let Some(class) = name.strip_suffix(".class") {
            if name.starts_with("META-INF/") {
                continue;
            }
            // An archive lists a package's classes together, so most
            // entries are the package the one before was in.
            if let Some((package, _)) = class.rsplit_once('/')
                && package != last_package
            {
                package.clone_into(&mut last_package);
                jar.packages.insert(package.replace('/', "."));
            }
        } else if !name.contains('/')
            && name.ends_with(".json")
            && name.contains("mixin")
            && !name.contains("refmap")
        {
            jar.mixin_configs.insert(name.to_string());
        } else if matches!(
            name,
            "fabric.mod.json"
                | "quilt.mod.json"
                | "META-INF/mods.toml"
                | "META-INF/neoforge.mods.toml"
        ) {
            metadata.push((index, name.to_string()));
        }
    }

    for (index, name) in metadata {
        let mut text = String::new();
        let Ok(mut entry) = archive.by_index(index) else {
            continue;
        };
        if entry
            .by_ref()
            .take(256 * 1024)
            .read_to_string(&mut text)
            .is_err()
        {
            continue;
        }

        if name.ends_with(".json") {
            read_json_metadata(&text, &mut jar);
        } else {
            read_toml_metadata(&text, &mut jar);
        }
    }

    Some(jar)
}

fn read_json_metadata(text: &str, jar: &mut ModJar) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };

    // Quilt nests the same things one level down.
    let loader = value.get("quilt_loader").unwrap_or(&value);
    if let Some(id) = loader.get("id").and_then(|id| id.as_str()) {
        jar.ids.push(id.to_string());
    }
    let name = loader
        .get("metadata")
        .unwrap_or(loader)
        .get("name")
        .and_then(|name| name.as_str());
    if let Some(name) = name
        && jar.name.is_empty()
    {
        jar.name = name.to_string();
    }

    let mixins = value.get("mixins").or_else(|| loader.get("mixin"));
    if let Some(mixins) = mixins.and_then(|mixins| mixins.as_array()) {
        for mixin in mixins {
            let config = mixin
                .as_str()
                .or_else(|| mixin.get("config").and_then(|c| c.as_str()));
            if let Some(config) = config {
                let config = config.rsplit('/').next().unwrap_or(config);
                jar.mixin_configs.insert(config.to_string());
            }
        }
    }
}

fn read_toml_metadata(text: &str, jar: &mut ModJar) {
    let Ok(value) = text.parse::<toml::Table>() else {
        return;
    };

    if let Some(mods) = value.get("mods").and_then(|mods| mods.as_array()) {
        for entry in mods {
            if let Some(id) = entry.get("modId").and_then(|id| id.as_str()) {
                jar.ids.push(id.to_string());
            }
            if let Some(name) =
                entry.get("displayName").and_then(|name| name.as_str())
                && jar.name.is_empty()
            {
                jar.name = name.to_string();
            }
        }
    }

    if let Some(mixins) = value.get("mixins").and_then(|m| m.as_array()) {
        for mixin in mixins {
            if let Some(config) = mixin.get("config").and_then(|c| c.as_str()) {
                let config = config.rsplit('/').next().unwrap_or(config);
                jar.mixin_configs.insert(config.to_string());
            }
        }
    }
}

/// The part of a crash that is about the crash.
///
/// A crash report goes on to list every mod and resource pack after the
/// exception, and a log is full of stack traces that were only ever warnings;
/// reading either whole would find every mod guilty of something. The game
/// prints its crash report into the log as it goes down, so that is looked for
/// first; failing it, the last exception the text holds.
pub fn crash_excerpt(text: &str) -> &str {
    if let Some(start) = text.rfind("---- Minecraft Crash Report ----") {
        let report = &text[start..];
        let end = report.find("-- System Details --").unwrap_or(report.len());
        return &report[..end];
    }

    let lines: Vec<(usize, &str)> = text
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let at = *offset;
            *offset += line.len();
            Some((at, line))
        })
        .collect();

    let is_frame = |line: &str| {
        let line = line.trim_start();
        line.starts_with("at ") || line.starts_with("... ")
    };

    // The last line that heads a trace: not a frame, not a cause, and followed
    // by a frame.
    let head = lines.windows(2).rposition(|pair| {
        let (_, line) = pair[0];
        let (_, next) = pair[1];
        !is_frame(line)
            && !line.trim_start().starts_with("Caused by")
            && is_frame(next)
    });

    head.map_or("", |index| &text[lines[index].0..])
}

static FRAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^\s*(?:at|j)\s+(?:[\w.\-]+(?:@[\w.\-]*)?/)*(?P<class>[\w$]+(?:\.[\w$]+)+)\.(?P<method>[\w$<>]+)\(",
    )
    .expect("frame pattern is valid")
});

/// Fabric names a mixin's injected method after the mod that owns it:
/// `handler$zzb000$sodium$onRender`.
static MIXIN_HANDLER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$[a-z]{3}\d{3}\$(?P<mod_id>[a-z][a-z0-9_]*)\$")
        .expect("mixin handler pattern is valid")
});

/// The mods a crash excerpt points at, most likely first.
///
/// The deepest cause is read first — that is where the failure started — and
/// within it the frames from the top down. A frame counts when the class it
/// runs is a mod's own, or when it is a method a mod's mixin put there.
pub fn suspects(excerpt: &str, index: &ModIndex, limit: usize) -> Vec<Suspect> {
    let mut found: Vec<Suspect> = Vec::new();

    let blocks: Vec<&str> = excerpt.split("Caused by").collect();
    for block in blocks.iter().rev() {
        for frame in FRAME.captures_iter(block) {
            if found.len() >= limit {
                return found;
            }

            let class = &frame["class"];
            let method = &frame["method"];

            let jar = if let Some(handler) = MIXIN_HANDLER.captures(method) {
                index.by_id(&handler["mod_id"])
            } else if is_framework(class) {
                None
            } else {
                index.by_symbol(class)
            };

            if let Some(jar) = jar
                && !found.iter().any(|suspect| suspect.file == jar.file)
            {
                found.push(Suspect {
                    file: jar.file.clone(),
                    name: jar.name.clone(),
                    clue: format!("{class}.{method}"),
                });
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jar(file: &str, name: &str, id: &str, packages: &[&str]) -> ModJar {
        ModJar {
            file: file.to_string(),
            name: name.to_string(),
            ids: vec![id.to_string()],
            packages: packages.iter().map(|p| p.to_string()).collect(),
            mixin_configs: HashSet::new(),
        }
    }

    fn index() -> ModIndex {
        ModIndex {
            jars: vec![
                jar(
                    "mods/fabric-api.jar",
                    "Fabric API",
                    "fabric-api",
                    &["net.fabricmc.fabric.impl"],
                ),
                jar(
                    "mods/create.jar",
                    "Create",
                    "create",
                    &["com.simibubi.create.content"],
                ),
                jar(
                    "mods/sodium.jar",
                    "Sodium",
                    "sodium",
                    &["net.caffeinemc.mods.sodium.client"],
                ),
            ],
            broken: Vec::new(),
        }
    }

    #[test]
    fn the_deepest_cause_names_the_mod() {
        let report = "---- Minecraft Crash Report ----\nDescription: Ticking entity\n\njava.lang.RuntimeException: wrapped\n\tat net.minecraft.world.level.Level.tick(Level.java:10)\nCaused by: java.lang.NullPointerException\n\tat net.fabricmc.fabric.impl.Event.invoke(Event.java:3)\n\tat com.simibubi.create.content.Belt.tick(Belt.java:40)\n\n-- System Details --\nMods: sodium, create";
        let found = suspects(crash_excerpt(report), &index(), 3);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file, "mods/create.jar");
        assert_eq!(found[0].clue, "com.simibubi.create.content.Belt.tick");
    }

    #[test]
    fn a_mixin_handler_names_its_mod() {
        let text = "java.lang.IllegalStateException: boom\n\tat net.minecraft.client.renderer.LevelRenderer.handler$zzb000$sodium$onRender(LevelRenderer.java:1)\n\tat net.minecraft.client.Minecraft.run(Minecraft.java:2)";
        let found = suspects(crash_excerpt(text), &index(), 3);

        assert_eq!(found[0].name, "Sodium");
    }

    #[test]
    fn a_mod_list_after_the_trace_is_not_evidence() {
        let report = "---- Minecraft Crash Report ----\njava.lang.OutOfMemoryError: Java heap space\n\tat java.util.Arrays.copyOf(Arrays.java:1)\n\n-- System Details --\n\tat com.simibubi.create.content.Belt.tick(Belt.java:40)";
        assert!(suspects(crash_excerpt(report), &index(), 3).is_empty());
    }

    #[test]
    fn a_log_is_read_from_its_last_trace() {
        let log = "[main/WARN]: harmless\njava.lang.Exception: old\n\tat com.simibubi.create.content.Belt.tick(Belt.java:1)\n[main/INFO]: later\njava.lang.NullPointerException: new\n\tat net.caffeinemc.mods.sodium.client.Render.draw(Render.java:2)\n";
        let found = suspects(crash_excerpt(log), &index(), 3);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Sodium");
    }

    #[test]
    fn a_symbol_resolves_to_the_jar_holding_its_package() {
        let index = index();
        assert_eq!(
            index
                .by_symbol("com/simibubi/create/content/Belt")
                .map(|jar| jar.name.as_str()),
            Some("Create")
        );
        assert_eq!(
            index
                .by_symbol("com.simibubi.create.content.Belt.tick")
                .map(|jar| jar.name.as_str()),
            Some("Create")
        );
        assert!(index.by_symbol("org.example.Missing").is_none());
    }
}
