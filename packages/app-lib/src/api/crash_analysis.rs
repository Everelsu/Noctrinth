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
const MAX_FINDINGS: usize = 8;

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CrashDiagnosis {
    pub findings: Vec<CrashFinding>,
    pub sources: Vec<CrashSourceFile>,
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
    /// Kinds of file this rule speaks about; empty means all of them.
    kinds: &'static [CrashSourceKind],
}

/// The rules, in the order their findings are shown when they tie on severity.
static RULES: &[Rule] = &[
    // What the game itself said, which is a headline rather than a diagnosis.
    Rule {
        id: "crash_description",
        severity: CrashSeverity::Note,
        pattern: r"(?m)^Description: (?P<description>.+)$",
        also: None,
        kinds: &[CrashSourceKind::CrashReport],
    },
    // Forge and NeoForge work some of this out themselves and say so.
    Rule {
        id: "loader_suggestion",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^\s*A potential solution has been determined[:,]?\s*(?P<suggestion>.*)$",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "suspected_mods",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^\s*Suspected Mods?: (?P<mods>.+)$",
        also: None,
        kinds: &[],
    },
    // Memory.
    Rule {
        id: "out_of_memory_heap",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.OutOfMemoryError: Java heap space",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "out_of_memory_metaspace",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.OutOfMemoryError: (?:Metaspace|Compressed class space)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "out_of_memory_system",
        severity: CrashSeverity::Critical,
        pattern: r"(?:There is insufficient memory for the Java Runtime Environment|Native memory allocation \(\w+\) failed|Failed to reserve shared memory)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "heap_too_large_to_start",
        severity: CrashSeverity::Critical,
        pattern: r"Could not reserve enough space for (?P<size>[\w ]+) object heap",
        also: None,
        kinds: &[],
    },
    // Java itself.
    Rule {
        id: "java_too_old",
        severity: CrashSeverity::Critical,
        pattern: r"class file version (?P<class_version>\d+)(?:\.\d+)?\), this version of the Java Runtime only recognizes class file versions up to (?P<runtime_version>\d+)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "java_unsupported_class",
        severity: CrashSeverity::Critical,
        pattern: r"java\.lang\.UnsupportedClassVersionError: (?P<class_name>[\w./$]+)",
        also: None,
        kinds: &[],
    },
    // Mods that are missing, doubled, or built for something else.
    Rule {
        id: "fabric_missing_dependency",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)^\s*-\s*Mod '(?P<mod_name>[^']+)' \((?P<mod_id>[^)]+)\)[^\n]*? requires [^\n]*? of (?:mod )?'?(?P<dependency>[\w\-.]+)'?, which is missing",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "fabric_wrong_dependency_version",
        severity: CrashSeverity::Critical,
        pattern: r"(?m)^\s*-\s*Mod '(?P<mod_name>[^']+)' \((?P<mod_id>[^)]+)\)[^\n]*? requires (?P<requirement>[^\n]+?) of (?:mod )?'?(?P<dependency>[\w\-.]+)'?, but only the wrong version",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "forge_missing_dependency",
        severity: CrashSeverity::Critical,
        pattern: r"Mod ID: '(?P<dependency>[^']+)', Requested by: '(?P<mod_id>[^']+)'",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "duplicate_mods",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)duplicate mod(?:s| ids| entries)?(?: found| detected)?[:!]?\s*(?P<mods>[^\n]*)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "mod_for_other_version",
        severity: CrashSeverity::Warning,
        pattern: r"java\.lang\.(?:NoSuchMethodError|NoClassDefFoundError|NoSuchFieldError): (?:Failed resolution of: )?(?P<symbol>[\w./$;()\[\]<>]*net[/.]minecraft[\w./$;()\[\]<>]*)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "mixin_failed",
        severity: CrashSeverity::Critical,
        pattern: r"Mixin (?:apply|prepare|transformation) (?:for|of)?\s*[^\n]*?(?P<config>[\w\-.]+\.mixins?\.json)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "optifine_present",
        severity: CrashSeverity::Warning,
        pattern: r"(?P<frame>(?:net\.optifine|optifine)[\w.$]*)",
        also: Some(r"(?i)(?:exception|error|crash)"),
        kinds: &[],
    },
    Rule {
        id: "connector_fabric_mod",
        severity: CrashSeverity::Warning,
        pattern: r"(?i)sinytra|connector",
        also: Some(r"(?i)(?:sodium|iris|indium)"),
        kinds: &[],
    },
    // The files under the game.
    Rule {
        id: "corrupted_archive",
        severity: CrashSeverity::Critical,
        pattern: r"(?:java\.util\.zip\.ZipException|Invalid CEN header|zip END header not found|error in opening zip file|java\.io\.EOFException)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "disk_full",
        severity: CrashSeverity::Critical,
        pattern: r"(?:No space left on device|There is not enough space on the disk|ENOSPC)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "file_locked",
        severity: CrashSeverity::Warning,
        pattern: r"The process cannot access the file because it is being used by another process",
        also: None,
        kinds: &[],
    },
    // The machine the game is drawn on.
    Rule {
        id: "gpu_driver_amd",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>ati[a-z0-9]*\.dll|amdvlk[a-z0-9]*\.dll|amdxx[a-z0-9]*\.dll)",
        also: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "gpu_driver_nvidia",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>nvoglv(?:32|64)\.dll|nvd3dum\.dll)",
        also: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "gpu_driver_intel",
        severity: CrashSeverity::Critical,
        pattern: r"(?i)(?P<library>ig[a-z0-9]*icd(?:32|64)\.dll|igd[a-z0-9]*\.dll)",
        also: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "opengl_unsupported",
        severity: CrashSeverity::Critical,
        pattern: r"(?:GLFW error 6554[0-9]|WGL: The driver does not appear to support OpenGL|Pixel format not accelerated|Failed to create window|OpenGL 3\.2|GL_ARB_framebuffer_object)",
        also: None,
        kinds: &[],
    },
    Rule {
        id: "missing_native_library",
        severity: CrashSeverity::Critical,
        pattern: r"(?:java\.lang\.UnsatisfiedLinkError|no lwjgl(?:64)? in java\.library\.path|Failed to locate library)",
        also: None,
        kinds: &[],
    },
    // The JVM died rather than the game, and said where.
    Rule {
        id: "jvm_problematic_frame",
        severity: CrashSeverity::Warning,
        pattern: r"(?m)^#\s*(?:C|J|V|j)\s+(?P<frame>.+)$",
        also: None,
        kinds: &[CrashSourceKind::JvmError],
    },
    Rule {
        id: "jvm_signal",
        severity: CrashSeverity::Note,
        pattern: r"(?m)^#\s*(?P<signal>(?:EXCEPTION_|SIG)[A-Z_]+) \(0x[0-9a-fA-F]+\)",
        also: None,
        kinds: &[CrashSourceKind::JvmError],
    },
];

static COMPILED: LazyLock<Vec<(Regex, Option<Regex>)>> = LazyLock::new(|| {
    RULES
        .iter()
        .map(|rule| {
            (
                Regex::new(rule.pattern)
                    .expect("a crash rule pattern is a valid regex"),
                rule.also.map(|also| {
                    Regex::new(also)
                        .expect("a crash rule pattern is a valid regex")
                }),
            )
        })
        .collect()
});

/// Everything the rules recognise in one file's worth of text.
///
/// A rule speaks once per file, on the first line it matched: a mixin failure
/// that took ten mods down with it is one thing that went wrong, not ten.
pub fn findings_in(
    text: &str,
    kind: CrashSourceKind,
    source_name: &str,
) -> Vec<CrashFinding> {
    let mut findings = Vec::new();

    for (rule, (pattern, also)) in RULES.iter().zip(COMPILED.iter()) {
        if !rule.kinds.is_empty() && !rule.kinds.contains(&kind) {
            continue;
        }

        if let Some(also) = also
            && !also.is_match(text)
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

    let instance_path: Option<String> =
        sqlx::query_scalar("SELECT path FROM instances WHERE id = ?")
            .bind(instance_id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(instance_path) = instance_path else {
        return Ok(CrashDiagnosis::default());
    };

    let instance_dir = state.directories.instances_dir().join(&instance_path);
    let logs_dir = state.directories.instance_logs_dir(&instance_path);
    let crash_reports_dir = state.directories.crash_reports_dir(&instance_path);

    let mut diagnosis = CrashDiagnosis::default();

    // The game's own report first: when there is one, it is about the run that
    // just ended and it names the exception.
    if let Some(report) = newest_file(&crash_reports_dir, |name| {
        name.starts_with("crash-") && name.ends_with(".txt")
    })
    .await
    {
        read_into(
            &mut diagnosis,
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
    {
        read_into(
            &mut diagnosis,
            &jvm_error,
            CrashSourceKind::JvmError,
            ReadFrom::Start(JVM_ERROR_HEAD_BYTES),
        )
        .await;
    }

    // And the log it was writing, which is where anything the other two missed
    // was printed on the way down.
    let latest_log = logs_dir.join("latest.log");
    if latest_log.is_file() {
        read_into(
            &mut diagnosis,
            &latest_log,
            CrashSourceKind::Log,
            ReadFrom::End(LOG_TAIL_BYTES),
        )
        .await;
    }

    finish(&mut diagnosis);
    Ok(diagnosis)
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
    };

    finish(&mut diagnosis);
    diagnosis
}

/// Worst first, and no more than a screenful.
fn finish(diagnosis: &mut CrashDiagnosis) {
    let order: Vec<&str> = RULES.iter().map(|rule| rule.id).collect();

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

    let mut seen = Vec::new();
    diagnosis.findings.retain(|finding| {
        if seen.contains(&finding.rule) {
            return false;
        }
        seen.push(finding.rule.clone());
        true
    });

    diagnosis.findings.truncate(MAX_FINDINGS);
}

enum ReadFrom {
    /// The first bytes of the file.
    Start(u64),
    /// The last bytes of the file.
    End(u64),
}

async fn read_into(
    diagnosis: &mut CrashDiagnosis,
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
                name,
                modified: modified_seconds(path).await,
            });
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
    fn nothing_recognised_is_no_findings_at_all() {
        let text = "[15:04:22] [main/INFO]: Stopping!";
        assert!(
            analyze_text(text, CrashSourceKind::Log, "latest.log").is_empty()
        );
    }
}
