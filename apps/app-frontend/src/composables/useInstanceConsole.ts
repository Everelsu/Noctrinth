import { createConsoleState } from '@modrinth/ui'

import {
	clear_log_buffer,
	clear_log_buffer_for_process,
	get_live_log_buffer,
	get_live_log_buffer_for_process,
	get_logs,
} from '@/helpers/logs'

type ConsoleState = ReturnType<typeof createConsoleState>

interface LogEntry {
	filename: string
	name?: string
	log_type: string
	output?: string | null
	age?: number
	live?: boolean
}

interface InstanceConsoleEntry {
	liveConsole: ConsoleState
	historicalConsole: ConsoleState
	historicalCache: Map<string, string>
	logList: LogEntry[] | null
}

const instances = new Map<string, InstanceConsoleEntry>()

function getOrCreate(instanceId: string): InstanceConsoleEntry {
	let entry = instances.get(instanceId)
	if (entry) return entry

	entry = {
		liveConsole: createConsoleState(),
		historicalConsole: createConsoleState(),
		historicalCache: new Map(),
		logList: null,
	}
	instances.set(instanceId, entry)
	return entry
}

async function hydrate(instanceId: string): Promise<void> {
	const entry = getOrCreate(instanceId)
	if (entry.liveConsole.output.value.length > 0) return

	const buffer = await get_live_log_buffer(instanceId)
	if (buffer) {
		entry.liveConsole.addLegacyLog(buffer)
	}
}

async function getHistoricalLogs(instanceId: string): Promise<LogEntry[]> {
	const entry = getOrCreate(instanceId)
	if (entry.logList) return entry.logList

	const logs: LogEntry[] = await get_logs(instanceId, true)
	entry.logList = logs

	for (const log of logs) {
		if (log.output) {
			entry.historicalCache.set(log.filename, log.output)
		}
	}

	return logs
}

function getHistoricalContent(instanceId: string, filename: string): string | undefined {
	return instances.get(instanceId)?.historicalCache.get(filename)
}

function invalidate(instanceId: string): void {
	const entry = instances.get(instanceId)
	if (!entry) return
	entry.historicalCache.clear()
	entry.logList = null
}

async function clearLive(instanceId: string): Promise<void> {
	const entry = getOrCreate(instanceId)
	entry.liveConsole.clear()
	await clear_log_buffer(instanceId).catch(() => {})
}

async function destroy(instanceId: string): Promise<void> {
	instances.delete(instanceId)
	await clear_log_buffer(instanceId).catch(() => {})
}

/**
 * The live console of one copy of an instance.
 *
 * An instance can be running twice, and the two have nothing to do with each
 * other: separate output, separate buffer, separate everything but the folder
 * they were started from.
 */
const processConsoles = new Map<string, ConsoleState>()

function getOrCreateProcess(processUuid: string): ConsoleState {
	let console = processConsoles.get(processUuid)
	if (!console) {
		console = createConsoleState()
		processConsoles.set(processUuid, console)
	}
	return console
}

export function useProcessConsole(processUuid: string) {
	const console = getOrCreateProcess(processUuid)

	return {
		console,
		hydrate: async () => {
			if (console.output.value.length > 0) return
			const buffer = await get_live_log_buffer_for_process(processUuid)
			if (buffer) console.addLegacyLog(buffer)
		},
		clear: async () => {
			console.clear()
			await clear_log_buffer_for_process(processUuid).catch((error) => {
				window.console.warn('Failed to clear the log of one copy:', error)
			})
		},
		forget: () => {
			processConsoles.delete(processUuid)
		},
	}
}

export function useInstanceConsole(instanceId: string) {
	const entry = getOrCreate(instanceId)
	return {
		liveConsole: entry.liveConsole,
		historicalConsole: entry.historicalConsole,
		hydrate: () => hydrate(instanceId),
		getHistoricalLogs: () => getHistoricalLogs(instanceId),
		getHistoricalContent: (filename: string) => getHistoricalContent(instanceId, filename),
		invalidate: () => invalidate(instanceId),
		clearLive: () => clearLive(instanceId),
		destroy: () => destroy(instanceId),
	}
}
