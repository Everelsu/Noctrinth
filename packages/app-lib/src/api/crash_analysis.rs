//! Reads what the game left behind after a bad run and says what went wrong.
//!
//! A crash leaves three kinds of trace: the game's own crash report, the log it
//! was writing at the time, and — when the JVM itself died rather than the game
//! — an `hs_err_pid*.log` beside them. Between them the cause is almost always
//! written down plainly, a few hundred lines from the end, in words that mean
//! nothing to the person reading them.
//!
//! This finds the lines that matter and names the cause. Nothing here writes
//! prose: a rule reports its own id and whatever it captured, and the words a
//! player reads live in the interface with the rest of the translations. That
//! is also what keeps this honest about what it knows — a rule either matched
//! or it did not.
//!
//! Everything is local. The launcher already sends a log to mclo.gs when asked,
//! which is the better answer for anything unusual; this is the answer for the
//! usual, and it works with no connection at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::State;
use crate::api::crash_culprits::{self, ModIndex};
use crate::state::MemorySettings;
use crate::util::io::{self, IOError};

/// How much of a log is worth reading.
///
/// The cause of a crash is written at the end, and a log that has been running
/// for hours can be tens of megabytes of chat and chunk noise before it.
const LOG_TAIL_BYTES: u64 = 512 * 1024;

/// How much of a JVM error file is worth reading, from the top.
///
/// Everything that says why it died — the signal, the problematic frame, the
/// failing library — is in the header. What follows is the state of every
/// thread in the process.
const JVM_ERROR_HEAD_BYTES: u64 = 96 * 1024;

/// How many findings are worth showing at once.
const MAX_FINDINGS: usize = 10;

/// How many mods a crash is allowed to blame at once. Past two, a list of
/// names stops being a lead and starts being the mod list.
const MAX_SUSPECTS: usize = 2;

/// Written into the launcher's log of a run when the player stopped it, so a
/// run that was ended on purpose is not read as one that crashed.
pub const STOPPED_FROM_LAUNCHER: &str = "# Stopped from the launcher";

/// The line the launcher ends its log of every run with.
static EXIT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^# Process exited with status: (?P<status>.+?)\s*$")
        .expect("exit line pattern is valid")
});

/// How far from the log a crash report may have been written and still be about
/// the same run.
///
/// Crash reports pile up in a folder and are never cleared, so the newest one
/// can be from a week ago while the game has run fine since. The game writes it
/// as it goes down, moments after the last line of the log; anything older than
/// this is somebody else's crash and saying otherwise would be worse than
/// saying nothing.
const SAME_RUN_SECONDS: u64 = 5 * 60;

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum CrashSeverity {
    /// Worth knowing, not the cause on its own.
    Note,
    /// Likely to be the cause, or to have made it worse.
    Warning,
    /// This is why the game is not running.
    Critical,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum CrashSourceKind {
    /// `crash-reports/crash-*.txt`, written by the game itself.
    CrashReport,
    /// `logs/latest.log`, or whichever log was asked about.
    Log,
    /// `hs_err_pid*.log`, written by the JVM when it died.
    JvmError,
    /// `logs/launcher_log.txt`: everything the game printed during the run,
    /// written by the launcher, which also records how the process ended.
    LauncherLog,
}

/// One thing that was recognised, and where.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrashFinding {
    /// Which rule matched. The interface has the words for it.
    pub rule: String,
    pub severity: CrashSeverity,
    pub source: CrashSourceKind,
    /// The file it was found in, by name.
    pub source_name: String,
    /// The line that matched, so the reader can see it for themselves.
    pub evidence: String,
    /// What the rule pulled out of that line, to fill in its text.
    pub values: BTreeMap<String, String>,
}

/// A file that was read, and how recently the game wrote it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrashSourceFile {
    pub kind: CrashSourceKind,
    pub name: String,
    /// Seconds since the epoch, or 0 when the filesystem would not say.
    pub modified: u64,
}

/// How the last run of the game ended.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrashExit {
    /// As the operating system reported it: `exit code: 0xcfffffff`.
    pub status: String,
    pub success: bool,
    /// The player stopped it from the launcher.
    pub stopped: bool,
}

impl CrashExit {
    /// A run that ended on its own, and not well.
    pub fn crashed(&self) -> bool {
        !self.success && !self.stopped
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CrashDiagnosis {
    pub findings: Vec<CrashFinding>,
    pub sources: Vec<CrashSourceFile>,
    /// Absent when the launcher has no record of the run ending.
    pub exit: Option<CrashExit>,
}

impl CrashDiagnosis {
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

/// One thing worth recognising.
struct Rule {
    id: &'static str,
    severity: CrashSeverity,
    /// The line that gives the rule away. Named captures are handed to the
    /// interface as the values its text is filled in with.
    pattern: &'static str,
    /// A second thing that has to be somewhere in the same file, when one line
    /// on its own would be too easy to mistake.
    also: Option<&'static str>,
    /// Something that, if it is in the same file, means this rule is wrong.
    ///
    /// A log records what is installed as readily as what went wrong, so a rule
    /// looking for a name finds it in the mod list of every run that was fine.
    /// This is how a rule says what it is not about — and a wrong finding costs
    /// more than a missing one, because somebody acts on it.
    unless: Option<&'static str>,
    /// Kinds of file this rule speaks about; empty means all of them.
    kinds: &'static [CrashSourceKind],
}

/// Rules about an exception, which a log is full of whether or not it is the
/// one the game died of: a mod checking whether another is installed throws a
/// `ClassNotFoundException` it expects, and prints it. These read only the
/// crash itself — see `crash_culprits::crash_excerpt`.
const EXCERPT_ONLY: &[&str] = &["missing_class", "mod_for_other_version"];

/// The rules, in the order their findings are shown when they tie on severity.
static RULES: &[Rule] = &[
    // What the game itself said, which is a headline rather than a diagnosis.
    Rule {
        id: "crash_description",
        severity: CrashSeverity::Note,
        pattern: r"(?m)^Description: (?P<description>.+)$",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::CrashReport],
    },
    // Forge and NeoForge work some of this out themselves and say so.
    Rule {
        id: "loader_suggestion",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^\s*A potential solution has been determined[:,]?\s*(?P<suggestion>.*)$",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "suspected_mods",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^\s*Suspected Mods?: (?P<mods>.+)$",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Memory.
    Rule {
        id: "out_of_memory_heap",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.OutOfMemoryError: Java heap space",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "out_of_memory_metaspace",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.OutOfMemoryError: (?:Metaspace|Compressed class space)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "out_of_memory_system",
        severity: CrashSeverity::Critical,
        pattern: r"(?:There is insufficient memory for the Java Runtime Environment|Native memory allocation \(\w+\) failed|Failed to reserve shared memory)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "heap_too_large_to_start",
        severity: CrashSeverity::Critical,
        pattern: r"Could not reserve enough space for (?P<size>[\w ]+) object heap",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Java itself.
    Rule {
        id: "java_too_old",
        severity: CrashSeverity::Critical,
        pattern: r"class file version (?P<class_version>\d+)(?:\.\d+)?\), this version of the Java Runtime only recognizes class file versions up to (?P<runtime_version>\d+)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "java_unsupported_class",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.UnsupportedClassVersionError: (?P<class_name>[\w./$]+)",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Mods that are missing, doubled, or built for something else.
    Rule {
        id: "fabric_missing_dependency",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)^\s*-\s*Mod '(?P<mod_name>[^']+)' \((?P<mod_id>[^)]+)\)[^\n]*? requires [^\n]*? of (?:mod )?'?(?P<dependency>[\w\-.]+)'?, which is missing",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "fabric_wrong_dependency_version",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)^\s*-\s*Mod '(?P<mod_name>[^']+)' \((?P<mod_id>[^)]+)\)[^\n]*? requires (?P<requirement>[^\n]+?) of (?:mod )?'?(?P<dependency>[\w\-.]+)'?, but only the wrong version",
        also: None,
        unless: None,
        kinds: &[],
    },
    // The game's own libraries missing from the loader's point of view: the
    // install is incomplete, not a mod.
    Rule {
        id: "install_incomplete",
        severity: CrashSeverity::Critical,
        pattern: r"Mod ID: '(?:minecraft|neoforge|forge)', Requested by: [^\n]*?Actual version: '\[MISSING\]'",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "forge_missing_dependency",
        severity: CrashSeverity::Critical,
        pattern: r"Mod ID: '(?P<dependency>[^']+)', Requested by: '(?P<mod_id>[^']+)'",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "duplicate_mods",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)duplicate mod(?:s| ids| entries)?(?: found| detected)?[:!]?\s*(?P<mods>[^\n]*)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "mod_for_other_version",
        severity: CrashSeverity::Warning,
        pattern: r"java\.lang\.(?:NoSuchMethodError|NoClassDefFoundError|NoSuchFieldError): [^\n]*?(?P<symbol>net[/.]minecraft[\w./$;()\[\]<>]*)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "mixin_failed",
        severity: CrashSeverity::Critical,
        pattern: r"Mixin (?:apply|prepare|transformation) (?:for|of)?\s*[^\n]*?(?P<config>[\w\-.]+\.mixins?\.json)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "neoforge_dependency_version",
        severity: CrashSeverity::Critical,
        pattern: r"Mod ID: '(?P<dependency>[^']+)', Requested by: '(?P<mod_id>[^']+)', Expected range: '(?P<expected>[^']+)', Actual version: '(?P<actual>[^']+)'",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "mod_incompatible",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)^\s*-\s*Mod '(?P<mod_name>[^']+)' \((?P<mod_id>[^)]+)\)[^\n]*? is incompatible with [^\n]*?'?(?P<conflict>[\w\-.]+)'?",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "not_a_mod_file",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<file>[\w\-. ]+\.jar)[^\n]{0,40}?is not a valid (?:mod|jar) file",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Mixins, which is what most mod conflicts actually look like.
    Rule {
        id: "mixin_injection_failed",
        severity: CrashSeverity::Critical,
        pattern: r"Critical injection failure: [^\n]*?(?P<config>[\w\-.]+\.mixins?\.json)[^\n]*?(?:->|::)?\s*(?P<mixin>[\w$]+)?",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "mixin_invalid_injection",
        severity: CrashSeverity::Critical,
        pattern: r"org\.spongepowered\.asm\.mixin\.[\w.]*?(?P<exception>InvalidInjectionException|InjectionError|MixinApplyError|MixinTransformerError)",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Java that is too new, which reads nothing like Java that is too old.
    Rule {
        id: "java_too_new",
        severity: CrashSeverity::Critical,
        pattern: r"Unsupported class file major version (?P<major>\d+)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "missing_main_class",
        severity: CrashSeverity::Critical,
        pattern: r"Could not find or load main class (?P<class_name>[\w./$]+)",
        also: None,
        unless: None,
        kinds: &[],
    },
    // Graphics, from the line the game writes about what it is drawing on.
    Rule {
        id: "software_renderer",
        severity: CrashSeverity::Critical,
        pattern: r"Renderer: '(?P<renderer>[^']*(?:Microsoft Basic Render|llvmpipe|GDI Generic|SwiftShader)[^']*)'",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "integrated_gpu_in_use",
        severity: CrashSeverity::Note,
        pattern: r"Renderer: '(?P<renderer>[^']*(?:Intel\(R\) (?:UHD|HD|Iris) Graphics|AMD Radeon\(TM\) Graphics)[^']*)'",
        also: None,
        unless: None,
        kinds: &[],
    },
    // The world, which is the other thing a player can lose.
    Rule {
        id: "datapack_blocked_world",
        severity: CrashSeverity::Critical,
        pattern: r"Errors in currently selected datapacks prevented the world from loading",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "chunk_unreadable",
        severity: CrashSeverity::Warning,
        pattern: r"(?i)Chunk file at \[?(?P<chunk>-?\d+,\s*-?\d+)\]? is (?P<problem>missing|in the wrong location)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "config_unreadable",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?:Failed to load config|Error parsing config|ConfigLoadingException)[^\n]{0,80}?(?P<file>[\w\-./]+\.(?:toml|json5?|cfg|properties))",
        also: None,
        unless: None,
        kinds: &[],
    },
    // A resource name the game will not accept, which on a Windows install is
    // almost always the player's own folder name.
    Rule {
        id: "resource_location_invalid",
        severity: CrashSeverity::Critical,
        pattern: r"ResourceLocationException: (?P<problem>[^\n]*?)(?:in path|in ID)?\s*(?P<value>[^\n]*)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "language_provider_mismatch",
        severity: CrashSeverity::Critical,
        pattern: r"(?:(?:needs|requires) language provider (?P<provider>[\w]+):(?P<wanted>[\d.,\[\])(]+)|Missing or unsupported mandatory dependencies)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "java_module_error",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.module\.(?P<exception>FindException|ResolutionException|InvalidModuleDescriptorException)(?:: (?P<detail>[^\n]+))?",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "oculus_without_embeddium",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)oculus[^\n]{0,60}?requires[^\n]{0,40}?(?:embeddium|rubidium)|net[/.]caffeinemc[/.]mods[/.]sodium[/.]api[/.](?:vertex[/.]buffer[/.]VertexBufferWriter|memory[/.]MemoryIntrinsics)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "missing_indium",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?:requires[^\n]{0,40}?indium|Indium is required|No fabric renderer found|fabric-renderer-api-v1[^\n]{0,60}?(?:missing|not (?:found|installed)))",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "too_many_block_ids",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?:Invalid id (?P<id>\d{4,})|maximum (?:block|item) id|too many (?:blocks|items|ids))",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "server_thread_stuck",
        severity: CrashSeverity::Critical,
        pattern: r"(?:A single server tick (?:took|has taken) (?P<seconds>[\d.]+) seconds|Considering it to be crashed, server will forcibly shutdown|watchdog[^\n]{0,40}?(?:deadlock|stuck))",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "feature_order_cycle",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?:Feature order cycle found|Cycle while building feature order|A feature cycle was found)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "optifine_present",
        severity: CrashSeverity::Warning,
        pattern: r"(?P<frame>(?:net\.optifine|optifine)[\w.$]*)",
        also: Some(r"(?i)(?:exception|error|crash)"),
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "connector_fabric_mod",
        severity: CrashSeverity::Warning,
        // Sinytra Connector failing, rather than Sinytra Connector existing.
        // The old rule looked for its name next to Sodium's or Iris's, and a
        // log names every mod that loaded — so it fired on runs where nothing
        // was wrong and told people to replace mods that were working. What it
        // looks for now is Connector saying it could not carry something.
        pattern: r"(?i)(?:dev\.su5ed\.sinytra|org\.sinytra)[\w.]*?(?P<exception>\w*(?:Exception|Error))|Connector (?:failed to|could not) [^\n]+",
        also: None,
        // The official NeoForge builds do not go through Connector at all. A
        // pack can carry both — Connector for something else entirely — and
        // blaming it for a crash it had no part in is the mistake this rule was
        // making before.
        unless: Some(
            r"(?i)(?:sodium|iris)[\w\-]*?neoforge|neoforge[\w\-]*?(?:sodium|iris)",
        ),
        kinds: &[],
    },
    // The files under the game.
    Rule {
        id: "corrupted_archive",
        severity: CrashSeverity::Critical,
        pattern: r"(?:java\.util\.zip\.ZipException|Invalid CEN header|zip END header not found|zip file is empty|error in opening zip file|Failed to create secure jar for|UnionFileSystem\$UncheckedIOException|java\.io\.EOFException)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "disk_full",
        severity: CrashSeverity::Critical,
        pattern: r"(?:No space left on device|There is not enough space on the disk|ENOSPC)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "file_locked",
        severity: CrashSeverity::Warning,
        pattern: r"(?:FileSystemException: (?P<file>[^\n]+?): )?The process cannot access the file because it is being used by another process",
        also: None,
        unless: None,
        kinds: &[],
    },
    // The machine the game is drawn on.
    Rule {
        id: "gpu_driver_amd",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>ati[a-z0-9]*\.dll|amdvlk[a-z0-9]*\.dll|amdxx[a-z0-9]*\.dll)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "gpu_driver_nvidia",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>nvoglv(?:32|64)\.dll|nvd3dum\.dll)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "gpu_driver_intel",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>ig[a-z0-9]*icd(?:32|64)\.dll|igd[a-z0-9]*\.dll)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "opengl_unsupported",
        severity: CrashSeverity::Critical,
        pattern: r"(?:GLFW error 6554[0-9]|WGL: The driver does not appear to support OpenGL|Pixel format not accelerated|Failed to create window|OpenGL 3\.2|GL_ARB_framebuffer_object)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "missing_native_library",
        severity: CrashSeverity::Critical,
        pattern: r"(?:java\.lang\.UnsatisfiedLinkError|no lwjgl(?:64)? in java\.library\.path|Failed to locate library)",
        also: None,
        unless: None,
        kinds: &[],
    },
    // The JVM died rather than the game, and said where.
    // Native frames that name their own cause, which the generic frame rule
    // below would only quote.
    Rule {
        id: "native_allocator",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>jemalloc[\w.]*\.(?:dll|so|dylib))",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "native_audio",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>(?:soft_)?oal[\w.]*\.dll|libopenal[\w.]*\.so|OpenAL[\w.]*\.dylib)|alc?[A-Z]\w*Cleanup",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "native_window_linux",
        severity: CrashSeverity::Critical,
        pattern: r"(?P<library>libglfw[\w.]*\.so|libX11[\w.]*\.so|libGLX[\w.]*\.so)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "native_shader_macos",
        severity: CrashSeverity::Critical,
        pattern: r"(?P<library>libGLProgrammability\.dylib|GLEngine)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "wrong_jdk_apple_silicon",
        severity: CrashSeverity::Critical,
        pattern: r"~StubRoutines::SafeFetch32",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "jvm_itself_failed",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^#\s*[CJV]\s+\[?(?P<library>jvm\.dll|libjvm\.so)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "jvm_problematic_frame",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^#\s*(?:C|J|V|j)\s+(?P<frame>.+)$",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "jvm_signal",
        severity: CrashSeverity::Note,
        pattern: r"(?m)^#\s*(?P<signal>(?:EXCEPTION_|SIG)[A-Z_]+) \(0x[0-9a-fA-F]+\)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    // A class that is not where a mod expected it. The Minecraft case is the
    // rule above; this is the one where the class belongs to another mod — an
    // addon built for a different Create, say — or to a mod that is not
    // installed at all. Which of the two it is, the jar index settles.
    Rule {
        id: "missing_class",
        severity: CrashSeverity::Critical,
        pattern: r"(?:java\.lang\.|throwables\.)(?P<exception>NoClassDefFoundError|ClassNotFoundException|NoSuchMethodError|NoSuchFieldError|ClassMetadataNotFoundException): [^\n]*?(?P<symbol>[a-z][\w$]*(?:[./][\w$]+){2,})",
        also: None,
        // Java's own messages for a class that failed to initialise say
        // nothing about which mod is missing; the JNA rule below owns that one.
        unless: Some(r"Could not initialize class com\.sun\.jna\."),
        kinds: &[],
    },
    // A server config lives in the world's own folder, not in `config/`, and
    // a game that went down while saving leaves it empty.
    Rule {
        id: "server_config_broken",
        severity: CrashSeverity::Critical,
        pattern: r"Failed loading config file (?P<file>[^\s]+) of type SERVER for modid (?P<mod_id>[\w\-]+)",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "config_truncated",
        severity: CrashSeverity::Critical,
        pattern: r"(?:ParsingException: Not enough data available|Failed loading config file (?P<file>[^\s]+))",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "ferritecore_neighbor_table",
        severity: CrashSeverity::Critical,
        pattern: r"UnsupportedOperationException: [^\n]{0,400}FerriteCore config",
        also: None,
        unless: None,
        kinds: &[],
    },
    // JNA unpacks a native library into the temporary folder on first use; an
    // antivirus or a locked-down temporary folder stops it.
    Rule {
        id: "jna_blocked",
        severity: CrashSeverity::Critical,
        pattern: r"NoClassDefFoundError: Could not initialize class com\.sun\.jna\.",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "kubejs_datapack",
        severity: CrashSeverity::Critical,
        pattern: r"Failed to parse (?P<file>[^\s]+) from pack KubeJS Resource Pack",
        also: None,
        unless: None,
        kinds: &[],
    },
    Rule {
        id: "gpu_driver_generic",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)(?:nglMultiDrawElementsBaseVertex|^#\s+C\s+0x0000|^#\s+C\s+\[glfw\.dll)",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    // Spark's profiler loads a native library that newer Java versions take
    // down with them.
    Rule {
        id: "spark_profiler_crash",
        severity: CrashSeverity::Critical,
        pattern: r"libasyncProfiler\.so",
        also: None,
        unless: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    // How the process ended, which only the launcher saw. Each is skipped when
    // the player stopped the run themselves.
    Rule {
        id: "exit_not_responding",
        severity: CrashSeverity::Critical,
        pattern: r"# Process exited with status: exit code: (?:0xcfffffff|-805306369)",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
    Rule {
        id: "exit_killed_by_system",
        severity: CrashSeverity::Critical,
        pattern: r"# Process exited with status: signal: 9 \(SIGKILL\)",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
    Rule {
        id: "exit_missing_system_library",
        severity: CrashSeverity::Critical,
        pattern: r"# Process exited with status: exit code: (?P<code>0xc0000135|0xc0000142|0xc000007b)",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
    Rule {
        id: "exit_stack_buffer_overrun",
        severity: CrashSeverity::Warning,
        pattern: r"# Process exited with status: exit code: 0xc0000409",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
    Rule {
        id: "exit_access_violation",
        severity: CrashSeverity::Warning,
        pattern: r"# Process exited with status: (?:exit code: 0xc0000005|signal: 11 \(SIGSEGV\))",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
    Rule {
        id: "exit_stack_overflow",
        severity: CrashSeverity::Warning,
        pattern: r"# Process exited with status: exit code: 0xc00000fd",
        also: None,
        unless: Some(STOPPED_FROM_LAUNCHER),
        kinds: &[CrashSourceKind::LauncherLog],
    },
];

type CompiledRule = (Regex, Option<Regex>, Option<Regex>);

static COMPILED: LazyLock<Vec<CompiledRule>> = LazyLock::new(|| {
    let compile = |pattern: &str| {
        Regex::new(pattern).expect("a crash rule pattern is a valid regex")
    };

    RULES
        .iter()
        .map(|rule| {
            (
                compile(rule.pattern),
                rule.also.map(&compile),
                rule.unless.map(&compile),
            )
        })
        .collect()
});

/// Everything the rules recognise in one file's worth of text.
///
/// A rule speaks once per file, on the first line it matched: a mixin failure
/// that took ten mods down with it is one thing that went wrong, not ten.
pub fn findings_in(
    full_text: &str,
    kind: CrashSourceKind,
    source_name: &str,
) -> Vec<CrashFinding> {
    let mut findings = Vec::new();

    for (rule, (pattern, also, unless)) in RULES.iter().zip(COMPILED.iter()) {
        if !rule.kinds.is_empty() && !rule.kinds.contains(&kind) {
            continue;
        }

        let text = if EXCERPT_ONLY.contains(&rule.id) {
            crash_culprits::crash_excerpt(full_text)
        } else {
            full_text
        };

        if let Some(also) = also
            && !also.is_match(text)
        {
            continue;
        }

        if let Some(unless) = unless
            && unless.is_match(text)
        {
            continue;
        }

        let Some(captures) = pattern.captures(text) else {
            continue;
        };

        let mut values = BTreeMap::new();
        for name in pattern.capture_names().flatten() {
            if let Some(value) = captures.name(name) {
                values.insert(
                    name.to_string(),
                    value.as_str().trim().to_string(),
                );
            }
        }

        findings.push(CrashFinding {
            rule: rule.id.to_string(),
            severity: rule.severity,
            source: kind,
            source_name: source_name.to_string(),
            evidence: evidence_line(
                text,
                captures.get(0).map_or(0, |m| m.start()),
            ),
            values,
        });
    }

    findings
}

/// The line the match sits on, trimmed and shortened to something readable.
fn evidence_line(text: &str, at: usize) -> String {
    let start = text[..at].rfind('\n').map_or(0, |index| index + 1);
    let end = text[at..].find('\n').map_or(text.len(), |index| at + index);

    let line = text[start..end].trim();
    if line.chars().count() <= 300 {
        return line.to_string();
    }

    line.chars().take(300).collect::<String>() + "…"
}

/// Reads what the last run left behind and says what it recognises.
#[tracing::instrument]
pub async fn analyze_instance(
    instance_id: &str,
) -> crate::Result<CrashDiagnosis> {
    let state = State::get().await?;

    let Some(context) =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
    else {
        return Ok(CrashDiagnosis::default());
    };
    let instance_path = context.instance.path.clone();

    let instance_dir = state.directories.instances_dir().join(&instance_path);
    let logs_dir = state.directories.instance_logs_dir(&instance_path);
    let crash_reports_dir = state.directories.crash_reports_dir(&instance_path);

    let mut diagnosis = CrashDiagnosis::default();
    let mut texts: Vec<(CrashSourceKind, String, String)> = Vec::new();

    // The launcher's own log is started over on every launch and ends with how
    // the process did, so it is what everything else is dated against. The
    // game's `latest.log` is not rewritten when Java fails before the game
    // starts, and an older one would describe somebody else's crash.
    let launcher_log = logs_dir.join("launcher_log.txt");
    let latest_log = logs_dir.join("latest.log");
    let anchor = if launcher_log.is_file() {
        Some(modified_seconds(&launcher_log).await)
    } else if latest_log.is_file() {
        Some(modified_seconds(&latest_log).await)
    } else {
        None
    };

    // The game's own report first, when it belongs to the same run: it names
    // the exception, which the log around it often does not.
    if let Some(report) = newest_file(&crash_reports_dir, |name| {
        name.starts_with("crash-") && name.ends_with(".txt")
    })
    .await
        && is_same_run(&report, anchor).await
    {
        read_into(
            &mut diagnosis,
            &mut texts,
            &report,
            CrashSourceKind::CrashReport,
            ReadFrom::Start(LOG_TAIL_BYTES),
        )
        .await;
    }

    // Then the JVM's, which exists only when the process died under the game.
    if let Some(jvm_error) = newest_file(&instance_dir, |name| {
        name.starts_with("hs_err_pid") && name.ends_with(".log")
    })
    .await
        && is_same_run(&jvm_error, anchor).await
    {
        read_into(
            &mut diagnosis,
            &mut texts,
            &jvm_error,
            CrashSourceKind::JvmError,
            ReadFrom::Start(JVM_ERROR_HEAD_BYTES),
        )
        .await;
    }

    if launcher_log.is_file() {
        read_into(
            &mut diagnosis,
            &mut texts,
            &launcher_log,
            CrashSourceKind::LauncherLog,
            ReadFrom::End(LOG_TAIL_BYTES),
        )
        .await;
    }

    // And the game's log, which is where anything the others missed was
    // printed on the way down.
    if latest_log.is_file() && is_same_run(&latest_log, anchor).await {
        read_into(
            &mut diagnosis,
            &mut texts,
            &latest_log,
            CrashSourceKind::Log,
            ReadFrom::End(LOG_TAIL_BYTES),
        )
        .await;
    }

    diagnosis.exit = texts
        .iter()
        .find(|(kind, _, _)| *kind == CrashSourceKind::LauncherLog)
        .and_then(|(_, _, text)| exit_of(text));
    let crashed = diagnosis.exit.as_ref().is_some_and(CrashExit::crashed);

    if crashed || !diagnosis.findings.is_empty() {
        let mods_dir = instance_dir.join("mods");
        let index =
            tokio::task::spawn_blocking(move || ModIndex::read(&mods_dir))
                .await
                .unwrap_or_default();

        attribute(&mut diagnosis, &texts, &index, crashed);
        advise_memory(
            &mut diagnosis,
            &instance_dir,
            context.launch_overrides.memory,
            &state,
        )
        .await;

        if crashed {
            find_empty_configs(&mut diagnosis, &instance_dir).await;
            compare_with_working_mods(
                &mut diagnosis,
                &state,
                &instance_path,
                &instance_dir,
            )
            .await;
            warn_about_unstable_cpu(&mut diagnosis);
        }
    }

    finish(&mut diagnosis);
    Ok(diagnosis)
}

/// How the run ended, from the line the launcher closed its log with.
fn exit_of(launcher_log: &str) -> Option<CrashExit> {
    let status = EXIT_LINE
        .captures_iter(launcher_log)
        .last()?
        .name("status")?
        .as_str()
        .to_string();

    Some(CrashExit {
        success: status == "exit code: 0" || status == "exit status: 0",
        stopped: launcher_log.contains(STOPPED_FROM_LAUNCHER),
        status,
    })
}

fn finding(
    rule: &str,
    severity: CrashSeverity,
    source_name: &str,
    evidence: String,
    values: &[(&str, String)],
) -> CrashFinding {
    CrashFinding {
        rule: rule.to_string(),
        severity,
        source: CrashSourceKind::LauncherLog,
        source_name: source_name.to_string(),
        evidence,
        values: values
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect(),
    }
}

/// Puts a file to every finding that names a mod some other way, and names the
/// mods the crash itself points at.
fn attribute(
    diagnosis: &mut CrashDiagnosis,
    texts: &[(CrashSourceKind, String, String)],
    index: &ModIndex,
    crashed: bool,
) {
    let duplicates = index.duplicate_ids();

    for finding in &mut diagnosis.findings {
        let values = &mut finding.values;

        let named = values
            .get("mod_id")
            .and_then(|id| index.by_id(id))
            .or_else(|| {
                values
                    .get("config")
                    .and_then(|config| index.by_mixin_config(config))
            });
        if let Some(jar) = named {
            values.insert("mod_file".into(), jar.file.clone());
            values.insert("mod_name".into(), jar.name.clone());
        }

        if let Some(owner) = values
            .get("symbol")
            .and_then(|symbol| index.by_symbol(symbol))
        {
            let (name, file) = (owner.name.clone(), owner.file.clone());
            values.insert("owner_name".into(), name);
            values.insert("owner_file".into(), file);
        }

        if finding.rule == "duplicate_mods" && !duplicates.is_empty() {
            let files: Vec<String> = duplicates
                .iter()
                .flat_map(|(_, jars)| jars.iter().map(|jar| jar.file.clone()))
                .collect();
            values.insert("files".into(), files.join(", "));
        }
    }

    if !crashed {
        return;
    }

    // What the crash itself points at: the game's report when there is one,
    // the log otherwise.
    let source = [
        CrashSourceKind::CrashReport,
        CrashSourceKind::LauncherLog,
        CrashSourceKind::Log,
        CrashSourceKind::JvmError,
    ]
    .iter()
    .find_map(|kind| {
        texts.iter().find(|(text_kind, _, text)| {
            text_kind == kind && !crash_culprits::crash_excerpt(text).is_empty()
        })
    });

    if let Some((kind, name, text)) = source {
        let excerpt = crash_culprits::crash_excerpt(text);
        let named: Vec<String> = diagnosis
            .findings
            .iter()
            .filter_map(|finding| finding.values.get("mod_file").cloned())
            .collect();

        for suspect in crash_culprits::suspects(excerpt, index, MAX_SUSPECTS)
            .into_iter()
            .filter(|suspect| !named.contains(&suspect.file))
        {
            let evidence = excerpt
                .lines()
                .find(|line| line.contains(suspect.clue.as_str()))
                .map_or_else(
                    || suspect.clue.clone(),
                    |line| line.trim().to_string(),
                );
            let mut suspect_finding = finding(
                "suspect_mod",
                CrashSeverity::Warning,
                name,
                evidence,
                &[
                    ("mod_file", suspect.file),
                    ("mod_name", suspect.name),
                    ("clue", suspect.clue),
                ],
            );
            suspect_finding.source = *kind;
            diagnosis.findings.push(suspect_finding);
        }
    }

    if !diagnosis
        .findings
        .iter()
        .any(|finding| finding.rule == "duplicate_mods")
    {
        for (id, jars) in duplicates.iter().take(2) {
            let files: Vec<&str> =
                jars.iter().map(|jar| jar.file.as_str()).collect();
            diagnosis.findings.push(finding(
                "duplicate_mod_files",
                CrashSeverity::Critical,
                "mods",
                files.join("\n"),
                &[
                    ("mod_id", id.clone()),
                    ("mod_name", jars[0].name.clone()),
                    ("files", files.join(", ")),
                ],
            ));
        }
    }

    for file in index.broken.iter().take(3) {
        diagnosis.findings.push(finding(
            "broken_mod_file",
            CrashSeverity::Critical,
            "mods",
            file.clone(),
            &[("mod_file", file.clone())],
        ));
    }
}

/// How much memory a pack of this many mods wants to load a world without
/// running out, in MiB.
///
/// Not a measurement: the usual advice for modded Minecraft, which is what
/// packs of these sizes turn out to need in practice.
pub fn recommended_memory_mb(mod_count: usize) -> u32 {
    match mod_count {
        0..100 => 4096,
        100..200 => 6144,
        200..300 => 8192,
        _ => 10240,
    }
}

/// The most that can be given to the game while leaving the system, and the
/// launcher, enough to run.
async fn memory_ceiling_mb() -> u32 {
    let total_mb = crate::api::jre::get_max_memory()
        .await
        .map_or(0, |kib| kib / 1024) as u32;
    total_mb.saturating_sub((total_mb / 4).max(3072))
}

async fn enabled_mod_files(mods_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(mods_dir).await else {
        return files;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".jar") {
            files.push(name);
        }
    }
    files.sort();
    files
}

/// The memory a launch uses when the instance does not set its own: the
/// global default, raised to what a pack of this size needs when the machine
/// has it.
///
/// A default is picked for a machine, not for a pack, and 4 GB that suits a
/// light one is not enough for three hundred mods. Loading the world is where
/// that shows: the game either runs out outright or spends so long collecting
/// garbage that Windows decides it has stopped responding and closes it.
pub async fn memory_for_pack(
    instance_dir: &Path,
    default: MemorySettings,
) -> MemorySettings {
    let mod_count = enabled_mod_files(&instance_dir.join("mods")).await.len();
    let wanted =
        recommended_memory_mb(mod_count).min(memory_ceiling_mb().await);

    if wanted > default.maximum {
        tracing::info!(
            "Raising memory from {} MB to {wanted} MB for a pack of {mod_count} mods",
            default.maximum
        );
        MemorySettings { maximum: wanted }
    } else {
        default
    }
}

/// Offers more memory where the crash is one more memory would have avoided.
async fn advise_memory(
    diagnosis: &mut CrashDiagnosis,
    instance_dir: &Path,
    configured: Option<MemorySettings>,
    state: &State,
) {
    const MEMORY_RULES: &[&str] = &[
        "out_of_memory_heap",
        "out_of_memory_metaspace",
        "exit_not_responding",
        "server_thread_stuck",
    ];

    if !diagnosis
        .findings
        .iter()
        .any(|finding| MEMORY_RULES.contains(&finding.rule.as_str()))
    {
        return;
    }

    let current = match configured {
        Some(memory) => memory.maximum,
        None => match crate::state::Settings::get(&state.pool).await {
            Ok(settings) => {
                memory_for_pack(instance_dir, settings.memory).await.maximum
            }
            Err(_) => return,
        },
    };

    let mod_count = enabled_mod_files(&instance_dir.join("mods")).await.len();
    // Past what the pack should need, one more step is still worth offering
    // while the machine has it: the rule of thumb is not the pack.
    let recommended = recommended_memory_mb(mod_count)
        .max(current + 2048)
        .min(memory_ceiling_mb().await);

    if recommended <= current {
        return;
    }

    for finding in &mut diagnosis.findings {
        if MEMORY_RULES.contains(&finding.rule.as_str()) {
            finding
                .values
                .insert("recommended_mb".into(), recommended.to_string());
            finding
                .values
                .insert("current_mb".into(), current.to_string());
            finding
                .values
                .insert("mod_count".into(), mod_count.to_string());
        }
    }
}

/// A config file that is empty or zeroed, which is what a game that went down
/// while saving leaves behind, and what a mod then refuses to start on.
///
/// Server configs are looked for too: those live in each world's
/// `serverconfig/`, and a broken one stops that world opening while every other
/// world loads fine.
async fn find_empty_configs(
    diagnosis: &mut CrashDiagnosis,
    instance_dir: &Path,
) {
    const CONFIG_EXTENSIONS: &[&str] =
        &["toml", "json", "json5", "cfg", "properties", "snbt"];

    let mut pending = vec![instance_dir.join("config")];
    if let Ok(mut worlds) =
        tokio::fs::read_dir(instance_dir.join("saves")).await
    {
        while let Ok(Some(world)) = worlds.next_entry().await {
            pending.push(world.path().join("serverconfig"));
        }
    }
    let mut empty = Vec::new();

    while let Some(dir) = pending.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            let is_config = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    CONFIG_EXTENSIONS.contains(&extension)
                });
            if !is_config {
                continue;
            }

            let Ok(bytes) = tokio::fs::read(&path).await else {
                continue;
            };
            if bytes
                .iter()
                .all(|byte| *byte == 0 || byte.is_ascii_whitespace())
            {
                let relative = path
                    .strip_prefix(instance_dir)
                    .map_or_else(|_| path.clone(), Path::to_path_buf);
                empty.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    empty.sort();
    for file in empty.into_iter().take(3) {
        diagnosis.findings.push(finding(
            "config_file_empty",
            CrashSeverity::Critical,
            "config",
            file.clone(),
            &[("file", file)],
        ));
    }
}

fn working_mods_path(state: &State, instance_path: &str) -> PathBuf {
    state
        .directories
        .caches_dir()
        .join("noctrinth-working-mods")
        .join(format!("{instance_path}.json"))
}

/// Writes down the mods an instance just ran with, after a run that ended
/// cleanly, so that the next crash can say what has changed since.
pub async fn remember_working_mods(instance_path: &str) {
    let Ok(state) = State::get().await else {
        return;
    };
    let instance_dir = state.directories.instances_dir().join(instance_path);
    let mods = enabled_mod_files(&instance_dir.join("mods")).await;
    let path = working_mods_path(&state, instance_path);

    let result = async {
        if let Some(parent) = path.parent() {
            io::create_dir_all(parent).await?;
        }
        io::write(&path, serde_json::to_vec(&mods)?).await?;
        Ok::<_, crate::Error>(())
    }
    .await;

    if let Err(error) = result {
        tracing::warn!(
            "Could not remember the mods of {instance_path}: {error}"
        );
    }
}

/// What was added to and taken out of `mods/` since the last run that ended
/// cleanly. A pack that worked yesterday and not today usually has its answer
/// here, and it is the one thing no log can say.
async fn compare_with_working_mods(
    diagnosis: &mut CrashDiagnosis,
    state: &State,
    instance_path: &str,
    instance_dir: &Path,
) {
    let Ok(bytes) =
        tokio::fs::read(working_mods_path(state, instance_path)).await
    else {
        return;
    };
    let Ok(working) = serde_json::from_slice::<Vec<String>>(&bytes) else {
        return;
    };
    let current = enabled_mod_files(&instance_dir.join("mods")).await;

    let added: Vec<&String> = current
        .iter()
        .filter(|mod_file| !working.contains(mod_file))
        .collect();
    let removed: Vec<&String> = working
        .iter()
        .filter(|mod_file| !current.contains(mod_file))
        .collect();
    if added.is_empty() && removed.is_empty() {
        return;
    }

    let list = |files: &[&String]| {
        let mut shown: Vec<String> =
            files.iter().take(8).map(|file| (*file).clone()).collect();
        if files.len() > 8 {
            shown.push(format!("+{}", files.len() - 8));
        }
        shown.join(", ")
    };

    let evidence = added
        .iter()
        .map(|file| format!("+ {file}"))
        .chain(removed.iter().map(|file| format!("- {file}")))
        .collect::<Vec<_>>()
        .join("\n");

    diagnosis.findings.push(finding(
        "mods_changed_since_working",
        CrashSeverity::Warning,
        "mods",
        evidence,
        &[
            ("added", list(&added)),
            ("removed", list(&removed)),
            ("added_count", added.len().to_string()),
            ("removed_count", removed.len().to_string()),
        ],
    ));
}

/// Intel's 13th and 14th generation desktop processors degrade under load
/// until they crash in ways that look like anything else: Java, the driver, a
/// random mod. When Java itself went down on one of those, it is worth saying,
/// because no amount of changing mods will fix it.
fn warn_about_unstable_cpu(diagnosis: &mut CrashDiagnosis) {
    static AFFECTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:13|14)th Gen Intel\(R\) Core\(TM\) (?P<model>i[579]-1[34]\d{2,3}(?:K|KF|KS|F|HX|T)?)(?:\s|$)",
        )
        .expect("cpu pattern is valid")
    });
    const NATIVE_CRASH_RULES: &[&str] = &[
        "jvm_itself_failed",
        "jvm_problematic_frame",
        "exit_access_violation",
        "exit_stack_buffer_overrun",
    ];

    if !diagnosis
        .findings
        .iter()
        .any(|finding| NATIVE_CRASH_RULES.contains(&finding.rule.as_str()))
    {
        return;
    }

    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::nothing()),
    );
    let Some(brand) = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
    else {
        return;
    };
    let Some(captures) = AFFECTED.captures(&brand) else {
        return;
    };
    let model = captures["model"].to_string();

    diagnosis.findings.push(finding(
        "intel_cpu_instability",
        CrashSeverity::Warning,
        "system",
        brand,
        &[("model", model)],
    ));
}

/// The same, for text the caller already has — a log the player is looking at,
/// or the console buffer of a run that has not been written out yet.
pub fn analyze_text(
    text: &str,
    kind: CrashSourceKind,
    source_name: &str,
) -> CrashDiagnosis {
    let mut diagnosis = CrashDiagnosis {
        findings: findings_in(text, kind, source_name),
        sources: vec![CrashSourceFile {
            kind,
            name: source_name.to_string(),
            modified: 0,
        }],
        exit: None,
    };

    finish(&mut diagnosis);
    diagnosis
}

/// The same, for a file of one instance's the player picked out — an older
/// crash report, say — read against that instance's mods, so the mod it points
/// at is named and can be switched off as it would be for the latest run.
pub async fn analyze_text_for_instance(
    instance_id: &str,
    text: &str,
    kind: CrashSourceKind,
    source_name: &str,
) -> crate::Result<CrashDiagnosis> {
    let mut diagnosis = CrashDiagnosis {
        findings: findings_in(text, kind, source_name),
        sources: vec![CrashSourceFile {
            kind,
            name: source_name.to_string(),
            modified: 0,
        }],
        exit: None,
    };

    let state = State::get().await?;
    if let Some(context) =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
    {
        let instance_dir = state
            .directories
            .instances_dir()
            .join(&context.instance.path);
        let mods_dir = instance_dir.join("mods");
        let index =
            tokio::task::spawn_blocking(move || ModIndex::read(&mods_dir))
                .await
                .unwrap_or_default();

        // A crash report is a crash by definition; any other file only gets
        // its findings given files, not a culprit read out of its traces.
        let texts = [(kind, source_name.to_string(), text.to_string())];
        attribute(
            &mut diagnosis,
            &texts,
            &index,
            kind == CrashSourceKind::CrashReport,
        );
        advise_memory(
            &mut diagnosis,
            &instance_dir,
            context.launch_overrides.memory,
            &state,
        )
        .await;
    }

    finish(&mut diagnosis);
    Ok(diagnosis)
}

/// Worst first, and no more than a screenful.
fn finish(diagnosis: &mut CrashDiagnosis) {
    let order: Vec<&str> = RULES.iter().map(|rule| rule.id).collect();

    // A class of the game's own is the rule for mods built for another
    // version, not for one mod missing another.
    diagnosis.findings.retain(|finding| {
        finding.rule != "missing_class"
            || !finding.values.get("symbol").is_some_and(|symbol| {
                ["net/minecraft", "net.minecraft", "com/mojang", "com.mojang"]
                    .iter()
                    .any(|prefix| symbol.starts_with(prefix))
            })
    });

    diagnosis.findings.sort_by(|a, b| {
        b.severity.cmp(&a.severity).then_with(|| {
            let index = |rule: &str| {
                order
                    .iter()
                    .position(|id| *id == rule)
                    .unwrap_or(usize::MAX)
            };
            index(&a.rule).cmp(&index(&b.rule))
        })
    });

    // A rule speaks once, except where it is about a file: two broken jars are
    // two things to fix.
    let mut seen = Vec::new();
    diagnosis.findings.retain(|finding| {
        let file = match finding.rule.as_str() {
            "suspect_mod" | "broken_mod_file" => finding.values.get("mod_file"),
            "config_file_empty" => finding.values.get("file"),
            _ => None,
        };
        let key = (finding.rule.clone(), file.cloned());
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
        true
    });

    // Two rules reading the same line are one thing that went wrong, told
    // twice. The sort above put the more specific rule first.
    let mut lines = Vec::new();
    diagnosis.findings.retain(|finding| {
        let key = (finding.source_name.clone(), finding.evidence.clone());
        if lines.contains(&key) {
            return false;
        }
        lines.push(key);
        true
    });

    diagnosis.findings.truncate(MAX_FINDINGS);
}

/// Whether a file was written closely enough to the log to be about the run
/// the log is of.
///
/// With no log to date it against there is nothing to compare, and the newest
/// report is the best guess there is.
async fn is_same_run(path: &Path, log_modified: Option<u64>) -> bool {
    let Some(log_modified) = log_modified else {
        return true;
    };

    let written = modified_seconds(path).await;
    if written == 0 || log_modified == 0 {
        return true;
    }

    written.abs_diff(log_modified) <= SAME_RUN_SECONDS
}

enum ReadFrom {
    /// The first bytes of the file.
    Start(u64),
    /// The last bytes of the file.
    End(u64),
}

async fn read_into(
    diagnosis: &mut CrashDiagnosis,
    texts: &mut Vec<(CrashSourceKind, String, String)>,
    path: &Path,
    kind: CrashSourceKind,
    from: ReadFrom,
) {
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().to_string());

    match read_part(path, from).await {
        Ok(text) => {
            diagnosis.findings.extend(findings_in(&text, kind, &name));
            diagnosis.sources.push(CrashSourceFile {
                kind,
                name: name.clone(),
                modified: modified_seconds(path).await,
            });
            texts.push((kind, name, text));
        }
        Err(error) => {
            // A file that cannot be read says nothing about the crash, and
            // failing the whole diagnosis over it would say even less.
            tracing::warn!(
                "Could not read {} for diagnosis: {error}",
                path.display()
            );
        }
    }
}

async fn read_part(path: &Path, from: ReadFrom) -> crate::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    let length = file
        .metadata()
        .await
        .map_err(|e| IOError::with_path(e, path))?
        .len();

    let (offset, wanted) = match from {
        ReadFrom::Start(wanted) => (0, wanted.min(length)),
        ReadFrom::End(wanted) => {
            (length.saturating_sub(wanted), wanted.min(length))
        }
    };

    if offset > 0 {
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| IOError::with_path(e, path))?;
    }

    let mut bytes = vec![0; wanted as usize];
    file.read_exact(&mut bytes)
        .await
        .map_err(|e| IOError::with_path(e, path))?;

    // A log is whatever encoding the machine writes in, and a crash is no time
    // to be strict about it.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn modified_seconds(path: &Path) -> u64 {
    let Ok(metadata) = io::metadata(path).await else {
        return 0;
    };

    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs())
}

/// The most recently written file in `dir` whose name the filter accepts.
async fn newest_file(
    dir: &Path,
    accept: impl Fn(&str) -> bool,
) -> Option<PathBuf> {
    let mut entries = tokio::fs::read_dir(dir).await.ok()?;
    let mut newest: Option<(u64, PathBuf)> = None;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Some(name) = path.file_name().map(|name| name.to_string_lossy())
        else {
            continue;
        };
        if !accept(name.as_ref()) {
            continue;
        }

        let modified = modified_seconds(&path).await;
        if newest.as_ref().is_none_or(|(at, _)| modified >= *at) {
            newest = Some((modified, path));
        }
    }

    newest.map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_matching(text: &str, kind: CrashSourceKind) -> Vec<String> {
        findings_in(text, kind, "test")
            .into_iter()
            .map(|finding| finding.rule)
            .collect()
    }

    #[test]
    fn every_rule_pattern_compiles() {
        assert_eq!(COMPILED.len(), RULES.len());
    }

    #[test]
    fn a_mod_list_is_not_a_diagnosis() {
        // What a perfectly healthy NeoForge run looks like: Connector installed
        // for something else, and the official builds of the rendering mods
        // beside it. Naming Connector here is what the rule used to do, and it
        // sent people to replace mods that were doing their job.
        let text = "[12:00:01] [main/INFO]: Loading 214 mods:\n\t- connectormod 1.0\n\t- sodium 0.6.0+mc1.21.1-neoforge\n\t- iris 1.8.0+mc1.21.1-neoforge\n[12:00:44] [Render thread/ERROR]: java.lang.OutOfMemoryError: Java heap space";
        let matched = rules_matching(text, CrashSourceKind::Log);

        assert!(!matched.contains(&"connector_fabric_mod".to_string()));
        assert!(matched.contains(&"out_of_memory_heap".to_string()));
    }

    #[test]
    fn connector_is_named_when_it_is_the_one_that_failed() {
        let text = "[12:00:03] [main/ERROR]: Connector failed to transform mod file sodium-fabric.jar\ndev.su5ed.sinytra.connector.ConnectorException: no";
        assert!(
            rules_matching(text, CrashSourceKind::Log)
                .contains(&"connector_fabric_mod".to_string())
        );
    }

    #[test]
    fn a_heap_that_ran_out_is_named() {
        let text = "[15:04:22] [Render thread/ERROR]: java.lang.OutOfMemoryError: Java heap space";
        assert!(
            rules_matching(text, CrashSourceKind::Log)
                .contains(&"out_of_memory_heap".to_string())
        );
    }

    #[test]
    fn a_missing_fabric_dependency_names_both_mods() {
        let text = "Incompatible mods found!\n\t- Mod 'Sodium' (sodium) 0.5.3 requires any version of fabric-api, which is missing!";
        let findings = findings_in(text, CrashSourceKind::Log, "latest.log");
        let finding = findings
            .iter()
            .find(|finding| finding.rule == "fabric_missing_dependency")
            .expect("the missing dependency should have been recognised");

        assert_eq!(finding.values["mod_name"], "Sodium");
        assert_eq!(finding.values["mod_id"], "sodium");
        assert_eq!(finding.values["dependency"], "fabric-api");
    }

    #[test]
    fn a_java_that_is_too_old_reports_both_versions() {
        let text = "java.lang.UnsupportedClassVersionError: com/example/Mod has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0";
        let findings = findings_in(text, CrashSourceKind::Log, "latest.log");
        let finding = findings
            .iter()
            .find(|finding| finding.rule == "java_too_old")
            .expect("the version mismatch should have been recognised");

        assert_eq!(finding.values["class_version"], "65");
        assert_eq!(finding.values["runtime_version"], "61");
    }

    #[test]
    fn a_driver_is_only_blamed_in_the_jvms_own_report() {
        let text = "# C  [nvoglv64.dll+0x8ad2f0]";

        assert!(
            rules_matching(text, CrashSourceKind::JvmError)
                .contains(&"gpu_driver_nvidia".to_string())
        );
        assert!(
            !rules_matching(text, CrashSourceKind::Log)
                .contains(&"gpu_driver_nvidia".to_string())
        );
    }

    #[test]
    fn a_rule_that_needs_a_second_signal_waits_for_it() {
        let quiet = "[12:00:00] [main/INFO]: Loading net.optifine.Config";
        let crashed = "[12:00:00] [main/INFO]: net.optifine.Config\njava.lang.RuntimeException: Mixin apply failed";

        assert!(
            !rules_matching(quiet, CrashSourceKind::Log)
                .contains(&"optifine_present".to_string())
        );
        assert!(
            rules_matching(crashed, CrashSourceKind::Log)
                .contains(&"optifine_present".to_string())
        );
    }

    #[test]
    fn the_worst_finding_is_the_first_one() {
        let text = "Description: Rendering overlay\njava.lang.OutOfMemoryError: Java heap space";
        let diagnosis =
            analyze_text(text, CrashSourceKind::CrashReport, "crash.txt");

        assert_eq!(diagnosis.findings[0].rule, "out_of_memory_heap");
        assert_eq!(diagnosis.findings[0].severity, CrashSeverity::Critical);
    }

    #[test]
    fn a_rule_speaks_once_per_diagnosis() {
        let text = "java.util.zip.ZipException: zip END header not found\njava.util.zip.ZipException: error in opening zip file";
        let diagnosis = analyze_text(text, CrashSourceKind::Log, "latest.log");

        assert_eq!(
            diagnosis
                .findings
                .iter()
                .filter(|finding| finding.rule == "corrupted_archive")
                .count(),
            1
        );
    }

    #[test]
    fn evidence_is_the_line_the_rule_read() {
        let text = "[15:04:22] [main/INFO]: starting\n[15:04:23] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space\n[15:04:24] [main/INFO]: stopping";
        let diagnosis = analyze_text(text, CrashSourceKind::Log, "latest.log");

        assert_eq!(
            diagnosis.findings[0].evidence,
            "[15:04:23] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space"
        );
    }

    #[test]
    fn a_game_windows_closed_for_not_responding_is_named() {
        let log = "[12:00:00] [main/INFO]: Loading world\n\n# Process exited with status: exit code: 0xcfffffff\n";
        assert!(
            rules_matching(log, CrashSourceKind::LauncherLog)
                .contains(&"exit_not_responding".to_string())
        );

        let exit = exit_of(log).expect("the exit line should be read");
        assert!(exit.crashed());
        assert_eq!(exit.status, "exit code: 0xcfffffff");
    }

    #[test]
    fn a_run_the_player_stopped_is_not_a_crash() {
        let log = "[12:00:00] [main/INFO]: Loading world\n\n# Stopped from the launcher\n\n# Process exited with status: signal: 9 (SIGKILL)\n";

        assert!(rules_matching(log, CrashSourceKind::LauncherLog).is_empty());
        assert!(
            !exit_of(log)
                .expect("the exit line should be read")
                .crashed()
        );
    }

    #[test]
    fn a_clean_exit_is_success() {
        let exit = exit_of("# Process exited with status: exit code: 0\n")
            .expect("the exit line should be read");
        assert!(exit.success && !exit.crashed());
    }

    #[test]
    fn a_missing_class_from_another_mod_is_named_and_the_games_is_not() {
        let addon = "java.lang.NoClassDefFoundError: com/simibubi/create/content/kinetics/base/KineticBlock\n\tat com.example.addon.Belt.<init>(Belt.java:1)";
        let diagnosis = analyze_text(addon, CrashSourceKind::Log, "latest.log");
        let finding = diagnosis
            .findings
            .iter()
            .find(|finding| finding.rule == "missing_class")
            .expect("the missing class should have been recognised");
        assert_eq!(
            finding.values["symbol"],
            "com/simibubi/create/content/kinetics/base/KineticBlock"
        );

        let vanilla = "java.lang.NoSuchMethodError: 'void net.minecraft.world.level.Level.tick()'\n\tat com.example.mod.Ticker.run(Ticker.java:1)";
        let rules: Vec<String> =
            analyze_text(vanilla, CrashSourceKind::Log, "latest.log")
                .findings
                .into_iter()
                .map(|finding| finding.rule)
                .collect();
        assert!(rules.contains(&"mod_for_other_version".to_string()));
        assert!(!rules.contains(&"missing_class".to_string()));
    }

    #[test]
    fn a_class_a_mod_only_checked_for_is_not_the_crash() {
        // A mod asking whether Xaero's Minimap is installed, early in a 1.7.10
        // log, and the crash that actually ended the run at the bottom.
        let log = "[10:00:01] [Client thread/INFO]: Looking for Xaero's Minimap\njava.lang.ClassNotFoundException: xaero.minimap.XaeroMinimap\n\tat java.net.URLClassLoader.findClass(URLClassLoader.java:1)\n\tat com.example.compat.Probe.check(Probe.java:2)\n[10:12:47] [Server thread/ERROR]: Encountered an unexpected exception\njava.lang.NullPointerException\n\tat com.uky.graves.GraveHandler.onTick(GraveHandler.java:3)\n";
        let rules: Vec<String> =
            analyze_text(log, CrashSourceKind::LauncherLog, "launcher_log.txt")
                .findings
                .into_iter()
                .map(|finding| finding.rule)
                .collect();

        assert!(!rules.contains(&"missing_class".to_string()));
    }

    #[test]
    fn one_line_is_one_finding() {
        let text = "Missing or unsupported mandatory dependencies:\n\tMod ID: 'minecraft', Requested by: 'create', Expected range: '[1.20.1]', Actual version: '[MISSING]'";
        let rules: Vec<String> =
            analyze_text(text, CrashSourceKind::Log, "latest.log")
                .findings
                .into_iter()
                .map(|finding| finding.rule)
                .collect();

        assert!(rules.contains(&"install_incomplete".to_string()));
        assert!(!rules.contains(&"forge_missing_dependency".to_string()));
    }

    #[test]
    fn a_bigger_pack_is_given_more_memory() {
        assert_eq!(recommended_memory_mb(40), 4096);
        assert_eq!(recommended_memory_mb(150), 6144);
        assert_eq!(recommended_memory_mb(250), 8192);
        assert_eq!(recommended_memory_mb(400), 10240);
    }

    #[test]
    fn nothing_recognised_is_no_findings_at_all() {
        let text = "[15:04:22] [main/INFO]: Stopping!";
        assert!(
            analyze_text(text, CrashSourceKind::Log, "latest.log").is_empty()
        );
    }
}
