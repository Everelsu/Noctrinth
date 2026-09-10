/**
 * The launcher's own reading of a crash.
 *
 * Upstream sends the live console to mclo.gs and shows what it makes of it,
 * which is the better answer for anything unusual — and no answer at all
 * without a connection, or for the crash report and the JVM's own error file,
 * which is where the cause is usually written in plain sight.
 *
 * This is the local half: rules that run against those files as well, on this
 * machine, and fixes that are pages of this launcher rather than advice. See
 * `packages/app-lib/src/api/crash_analysis.rs` for why the words a player reads
 * are not in the Rust.
 */
import { invoke } from '@tauri-apps/api/core'

export type CrashSeverity = 'note' | 'warning' | 'critical'
export type CrashSourceKind = 'crash_report' | 'log' | 'jvm_error'

export interface CrashFinding {
	rule: string
	severity: CrashSeverity
	source: CrashSourceKind
	source_name: string
	evidence: string
	values: Record<string, string>
}

export interface CrashSourceFile {
	kind: CrashSourceKind
	name: string
	modified: number
}

export interface CrashDiagnosis {
	findings: CrashFinding[]
	sources: CrashSourceFile[]
}

const EMPTY: CrashDiagnosis = { findings: [], sources: [] }

/** Reads the files the last run left behind and reports what it recognised. */
export async function analyzeInstanceCrash(instanceId: string): Promise<CrashDiagnosis> {
	try {
		return await invoke<CrashDiagnosis>('plugin:noctrinth-crash|crash_analyze_instance', {
			instanceId,
		})
	} catch (error) {
		// A diagnosis that cannot be made is not worth a notification: the log
		// the player came here to read is on screen either way.
		console.warn('Could not read the crash files for a diagnosis', error)
		return EMPTY
	}
}

/** The same rules, against text already in hand. */
export async function analyzeCrashText(
	text: string,
	kind: CrashSourceKind,
	sourceName: string,
): Promise<CrashDiagnosis> {
	if (!text.trim()) return EMPTY

	try {
		return await invoke<CrashDiagnosis>('plugin:noctrinth-crash|crash_analyze_text', {
			text,
			kind,
			sourceName,
		})
	} catch (error) {
		console.warn('Could not analyse the log', error)
		return EMPTY
	}
}

/** Which of two diagnoses to lead with, worst first. */
const SEVERITY_ORDER: Record<CrashSeverity, number> = {
	critical: 0,
	warning: 1,
	note: 2,
}

export function sortFindings(findings: CrashFinding[]): CrashFinding[] {
	return [...findings].sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity])
}
