//! The launcher's own reading of what a bad run left behind.
//!
//! A plugin of the fork's own so that upstream's stay upstream's. What it
//! exposes is the analysis in `theseus::crash_analysis` — see that module for
//! why the words a player reads are not in it.

use crate::api::Result;
use theseus::crash_analysis::{self, CrashDiagnosis, CrashSourceKind};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-crash")
        .invoke_handler(tauri::generate_handler![
            crash_analyze_instance,
            crash_analyze_text,
        ])
        .build()
}

/// Reads the crash report, the JVM's own error file and the tail of the log an
/// instance last wrote, and reports what it recognised.
#[tauri::command]
pub async fn crash_analyze_instance(
    instance_id: &str,
) -> Result<CrashDiagnosis> {
    Ok(crash_analysis::analyze_instance(instance_id).await?)
}

/// The same rules, against text the interface already has in hand: the log
/// being looked at, or the console of a run whose files are not written yet.
#[tauri::command]
pub async fn crash_analyze_text(
    text: String,
    kind: CrashSourceKind,
    source_name: String,
) -> Result<CrashDiagnosis> {
    Ok(crash_analysis::analyze_text(&text, kind, &source_name))
}
