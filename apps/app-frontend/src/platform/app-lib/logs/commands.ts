import { invoke } from '@tauri-apps/api/core'

import type { CrashDiagnosis } from '@/helpers/noctrinth-crash'

export interface HistoricalLog {
	filename: string
	log_type: string
	live?: boolean
}

export interface HistoricalLogCursor {
	before: number
	stamp: string
}

export interface HistoricalLogPage {
	output: string
	cursor: HistoricalLogCursor | null
	reset: boolean
}

export function getHistoricalPage(
	instanceId: string,
	log: HistoricalLog,
	cursor: HistoricalLogCursor | null,
): Promise<HistoricalLogPage> {
	return invoke('plugin:logs|logs_get_historical_page', {
		instanceId,
		logType: log.log_type,
		filename: log.filename,
		cursor,
	})
}

export function getHistoricalText(instanceId: string, log: HistoricalLog): Promise<string> {
	return invoke('plugin:logs|logs_get_output_by_filename', {
		instanceId,
		logType: log.log_type,
		filename: log.filename,
	})
}

export function analyzeHistoricalLog(
	instanceId: string,
	log: HistoricalLog,
): Promise<CrashDiagnosis> {
	return invoke('plugin:logs|logs_analyze_historical', {
		instanceId,
		logType: log.log_type,
		filename: log.filename,
	})
}
