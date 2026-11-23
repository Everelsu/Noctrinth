import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useStorage } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import {
	get as getInstance,
	get_content_items,
	list as listInstances,
	remove_project,
	run as runInstance,
	toggle_disable_project,
} from '@/helpers/instance'
import { get_logs } from '@/helpers/logs.js'

import type {
	ActivePluginModal,
	ActivePluginPanel,
	CatalogPluginEntry,
	InstalledPlugin,
	NoctrinthActionSlot,
	NoctrinthContentSourceSearchResult,
	NoctrinthPluginEvent,
	NoctrinthPluginModalRequest,
	NoctrinthPluginPageDeclaration,
	NoctrinthPluginPanelRequest,
	PluginCatalog,
	PluginLogEntry,
	RegisteredPluginAction,
	RegisteredPluginContentSource,
	RegisteredPluginPage,
} from './types'

export const DEFAULT_PLUGIN_CATALOG_URL = 'https://everelsu.github.io/Noctrinth-Plugins/index.json'

export const pluginCatalogUrl = useStorage(
	'noctrinth:plugin-catalog-url',
	DEFAULT_PLUGIN_CATALOG_URL,
)

export const quickCssEnabled = useStorage('noctrinth:quick-css-enabled', false)
export const quickCssContent = useStorage('noctrinth:quick-css-content', '')

export const installedPlugins = ref<InstalledPlugin[]>([])
export const catalogPlugins = ref<CatalogPluginEntry[]>([])
export const pluginRuntimeErrors = ref<Record<string, string>>({})
export const pluginLogs = ref<Record<string, PluginLogEntry[]>>({})
export const pluginActions = ref<RegisteredPluginAction[]>([])
export const pluginContentSources = ref<RegisteredPluginContentSource[]>([])
export const pluginPages = ref<RegisteredPluginPage[]>([])
export const activePluginModal = ref<ActivePluginModal | null>(null)
export const activePluginPanel = ref<ActivePluginPanel | null>(null)
export const pluginsSafeMode = ref(false)

const MAX_ERRORS_BEFORE_DISABLE = 3
const RPC_WINDOW_MS = 1000
const MAX_RPC_PER_WINDOW = 60
const FLOOD_KILL_RPC_PER_WINDOW = 200
const NOTIFY_WINDOW_MS = 10_000
const MAX_NOTIFY_PER_WINDOW = 6
const LOG_WINDOW_MS = 1000
const MAX_LOGS_PER_WINDOW = 40

export const pluginUpdates = computed<CatalogPluginEntry[]>(() => {
	const installedById = new Map(installedPlugins.value.map((p) => [p.manifest.id, p]))
	return catalogPlugins.value.filter((entry) => {
		const inst = installedById.get(entry.id)
		return inst && inst.manifest.version !== entry.version
	})
})

const MAX_PLUGIN_LOGS = 100

function appendPluginLog(pluginId: string, level: PluginLogEntry['level'], message: string) {
	const current = pluginLogs.value[pluginId] ?? []
	const next = [...current, { timestamp: Date.now(), level, message }].slice(-MAX_PLUGIN_LOGS)
	pluginLogs.value = { ...pluginLogs.value, [pluginId]: next }
}

export function clearPluginLogs(pluginId: string): void {
	const { [pluginId]: _removed, ...rest } = pluginLogs.value
	pluginLogs.value = rest
}

export function resolvePluginModal(
	actionId: string | null,
	values: Record<string, string> = {},
): void {
	const current = activePluginModal.value
	if (!current) return
	activePluginModal.value = null
	if (current.mode === 'form') {
		current.resolve({ action: actionId, values })
	} else {
		current.resolve(actionId)
	}
}

const panelMessageSinks = new Map<string, (data: unknown) => void>()

export function registerPluginPanelSink(
	pluginId: string,
	panelId: string,
	sink: (data: unknown) => void,
): void {
	panelMessageSinks.set(`${pluginId}:${panelId}`, sink)
}

export function unregisterPluginPanelSink(pluginId: string, panelId: string): void {
	panelMessageSinks.delete(`${pluginId}:${panelId}`)
}

export function closePluginPanel(): void {
	activePluginPanel.value = null
}

export function sendMessageFromPluginPanel(pluginId: string, panelId: string, data: unknown): void {
	const entry = activeWorkers.get(pluginId)
	if (!entry) return
	entry.worker.postMessage({ type: 'panel:message', panelId, data })
}

export function buildSandboxedPanelSrcdoc(html: string, pluginId: string, panelId: string): string {
	const themeVars = [
		'--color-bg',
		'--color-raised-bg',
		'--color-super-raised-bg',
		'--color-brand',
		'--color-brand-highlight',
		'--color-contrast',
		'--color-base',
		'--color-secondary',
		'--color-button-bg',
		'--color-button-border',
		'--color-divider',
		'--surface-1',
		'--surface-2',
		'--surface-3',
		'--surface-4',
		'--surface-5',
	]
	let rootVarCss = ''
	if (typeof document !== 'undefined' && typeof getComputedStyle === 'function') {
		const cs = getComputedStyle(document.documentElement)
		const pairs: string[] = []
		for (const name of themeVars) {
			const val = cs.getPropertyValue(name).trim()
			if (val) pairs.push(`${name}: ${val};`)
		}
		if (pairs.length > 0) {
			rootVarCss = `:root { ${pairs.join(' ')} }\n`
		}
	}

	const bridge = `<style>
${rootVarCss}html, body {
	margin: 0;
	padding: 0;
	background: var(--color-bg, #111017);
	color: var(--color-contrast, #f8f7fc);
	font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
	color-scheme: dark;
}
* { box-sizing: border-box; }
</style>
<script>
(function () {
	const listeners = [];
	const meta = ${JSON.stringify({ pluginId, panelId })};
	window.addEventListener('message', function (ev) {
		if (!ev.data || ev.data.__noctrinthHost !== true) return;
		for (let i = 0; i < listeners.length; i++) {
			try { listeners[i](ev.data.data); } catch (e) { console.error(e); }
		}
	});
	window.noctrinthPanel = Object.freeze({
		pluginId: meta.pluginId,
		panelId: meta.panelId,
		send: function (data) {
			window.parent.postMessage({ __noctrinthPanel: true, pluginId: meta.pluginId, panelId: meta.panelId, type: 'message', data: data }, '*');
		},
		onMessage: function (fn) {
			if (typeof fn === 'function') listeners.push(fn);
		},
		close: function () {
			window.parent.postMessage({ __noctrinthPanel: true, pluginId: meta.pluginId, panelId: meta.panelId, type: 'close' }, '*');
		}
	});
})();
</script>`

	if (/<head[\s>]/i.test(html)) {
		return html.replace(/<head([^>]*)>/i, `<head$1>${bridge}`)
	}
	return `<!doctype html><html><head><meta charset="utf-8">${bridge}</head><body>${html}</body></html>`
}

function applyQuickCss(): void {
	if (typeof document === 'undefined') return
	const existing = document.getElementById('noctrinth-quick-css')
	const css = !pluginsSafeMode.value && quickCssEnabled.value ? quickCssContent.value.trim() : ''
	if (!css) {
		existing?.remove()
		return
	}
	const styleEl = existing ?? document.createElement('style')
	styleEl.id = 'noctrinth-quick-css'
	styleEl.textContent = css
	if (!existing) document.head.appendChild(styleEl)
}

watch([quickCssEnabled, quickCssContent, pluginsSafeMode], applyQuickCss)

type NotifyFn = (opts: {
	title: string
	text?: string
	type?: 'info' | 'success' | 'warning' | 'error'
}) => void

let notificationHandler: NotifyFn | null = null

export function setPluginNotificationHandler(handler: NotifyFn) {
	notificationHandler = handler
}

interface WorkerEntry {
	worker: Worker
	blobUrl: string
	events: Set<string>
	pendingHostCalls: Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>
}

const activeWorkers = new Map<string, WorkerEntry>()
const appliedSettingAttrs = new Map<string, { attrs: string[]; vars: string[] }>()
let nextHostCallId = 1

function settingsStorageKey(pluginId: string): string {
	return `noctrinth:plugin-settings:${pluginId}`
}

function sanitizeToken(raw: string): string {
	return raw.toLowerCase().replace(/[^a-z0-9-]/g, '-')
}

export function getPluginSettings(plugin: InstalledPlugin): Record<string, unknown> {
	const result: Record<string, unknown> = {}
	for (const field of plugin.manifest.settings ?? []) {
		if (field.default !== undefined) result[field.key] = field.default
	}
	try {
		const raw = localStorage.getItem(settingsStorageKey(plugin.manifest.id))
		if (raw) {
			const parsed = JSON.parse(raw) as Record<string, unknown>
			Object.assign(result, parsed)
		}
	} catch {
		// Ignore malformed local storage
	}
	return result
}

function removePluginSettingVars(pluginId: string): void {
	if (typeof document === 'undefined') return
	const tracked = appliedSettingAttrs.get(pluginId)
	if (!tracked) return
	const root = document.documentElement
	for (const attr of tracked.attrs) root.removeAttribute(attr)
	for (const cssVar of tracked.vars) root.style.removeProperty(cssVar)
	appliedSettingAttrs.delete(pluginId)
}

function applyPluginSettingVars(plugin: InstalledPlugin): void {
	if (typeof document === 'undefined') return
	const id = plugin.manifest.id
	removePluginSettingVars(id)
	const fields = plugin.manifest.settings ?? []
	if (fields.length === 0) return

	const values = getPluginSettings(plugin)
	const root = document.documentElement
	const attrs: string[] = []
	const vars: string[] = []
	const safeId = sanitizeToken(id)

	for (const field of fields) {
		const val = values[field.key]
		if (val === undefined || val === null || val === '') continue
		const strVal = String(val)
		const safeKey = sanitizeToken(field.key)

		const attrName = `data-plugin-${safeId}-${safeKey}`
		root.setAttribute(attrName, strVal)
		attrs.push(attrName)

		const autoVar = `--noctrinth-plugin-${safeId}-${safeKey}`
		root.style.setProperty(autoVar, strVal)
		vars.push(autoVar)

		if (field.cssVar && /^--[a-zA-Z0-9-]+$/.test(field.cssVar)) {
			root.style.setProperty(field.cssVar, strVal)
			vars.push(field.cssVar)
		}
	}

	appliedSettingAttrs.set(id, { attrs, vars })
}

export function setPluginSetting(pluginId: string, key: string, value: unknown): void {
	let current: Record<string, unknown> = {}
	try {
		const raw = localStorage.getItem(settingsStorageKey(pluginId))
		if (raw) current = JSON.parse(raw) as Record<string, unknown>
	} catch {
		current = {}
	}
	current[key] = value
	localStorage.setItem(settingsStorageKey(pluginId), JSON.stringify(current))

	const plugin = installedPlugins.value.find((p) => p.manifest.id === pluginId)
	if (plugin?.enabled) {
		applyPluginSettingVars(plugin)
	}

	const entry = activeWorkers.get(pluginId)
	if (entry) {
		entry.worker.postMessage({
			type: 'event',
			event: 'settings:change',
			payload: { key, value, settings: current },
		})
	}
}

export function applyPluginPreset(
	pluginId: string,
	values: Record<string, string | boolean | number>,
): void {
	let current: Record<string, unknown> = {}
	try {
		const raw = localStorage.getItem(settingsStorageKey(pluginId))
		if (raw) current = JSON.parse(raw) as Record<string, unknown>
	} catch {
		current = {}
	}
	Object.assign(current, values)
	localStorage.setItem(settingsStorageKey(pluginId), JSON.stringify(current))

	const plugin = installedPlugins.value.find((p) => p.manifest.id === pluginId)
	if (plugin?.enabled) {
		applyPluginSettingVars(plugin)
	}

	const entry = activeWorkers.get(pluginId)
	if (entry) {
		entry.worker.postMessage({
			type: 'event',
			event: 'settings:change',
			payload: { key: '*', value: values, settings: current },
		})
	}
}

export async function listInstalledPlugins(): Promise<InstalledPlugin[]> {
	const list = await invoke<InstalledPlugin[]>('plugin:noctrinth-plugins|plugins_list')
	installedPlugins.value = list
	return list
}

export async function installPluginZip(zipPath: string): Promise<InstalledPlugin> {
	const plugin = await invoke<InstalledPlugin>('plugin:noctrinth-plugins|plugins_install_zip', {
		zipPath,
	})
	await listInstalledPlugins()
	await activatePlugin(plugin)
	return plugin
}

export async function installPluginRemote(
	url: string,
	expectedSha256: string,
): Promise<InstalledPlugin> {
	const plugin = await invoke<InstalledPlugin>('plugin:noctrinth-plugins|plugins_install_remote', {
		url,
		expectedSha256,
	})
	await listInstalledPlugins()
	await activatePlugin(plugin)
	return plugin
}

export async function installPluginById(id: string): Promise<InstalledPlugin> {
	let entry = catalogPlugins.value.find((p) => p.id === id)
	if (!entry) {
		const catalog = await fetchPluginCatalog()
		entry = catalog.plugins.find((p) => p.id === id)
	}
	if (!entry) {
		throw new Error(`Plugin "${id}" was not found in the catalog`)
	}
	return await installPluginRemote(entry.download_url, entry.sha256)
}

export async function setPluginEnabled(id: string, enabled: boolean): Promise<void> {
	await invoke('plugin:noctrinth-plugins|plugins_set_enabled', { id, enabled })
	const list = await listInstalledPlugins()
	const plugin = list.find((p) => p.manifest.id === id)
	if (!plugin) return
	if (enabled) {
		await activatePlugin(plugin)
	} else {
		deactivatePlugin(id)
	}
}

export async function reloadPlugin(id: string): Promise<void> {
	const list = await listInstalledPlugins()
	const plugin = list.find((p) => p.manifest.id === id)
	deactivatePlugin(id)
	if (plugin?.enabled) {
		await activatePlugin(plugin)
	}
}

export async function removePlugin(id: string): Promise<void> {
	deactivatePlugin(id)
	await invoke('plugin:noctrinth-plugins|plugins_remove', { id })
	const { [id]: _removed, ...rest } = pluginRuntimeErrors.value
	pluginRuntimeErrors.value = rest
	clearPluginLogs(id)
	await listInstalledPlugins()
}

export async function getPluginsDir(): Promise<string> {
	return await invoke<string>('plugin:noctrinth-plugins|plugins_get_dir')
}

export async function fetchPluginCatalog(url = pluginCatalogUrl.value): Promise<PluginCatalog> {
	const raw = await invoke<string>('plugin:noctrinth-plugins|plugins_fetch_catalog', {
		catalogUrl: url.trim() || DEFAULT_PLUGIN_CATALOG_URL,
	})
	const parsed = JSON.parse(raw) as PluginCatalog
	if (!parsed || !Array.isArray(parsed.plugins)) {
		throw new Error('Invalid plugin catalog format')
	}
	catalogPlugins.value = parsed.plugins
	return parsed
}

export async function checkPluginUpdates(): Promise<CatalogPluginEntry[]> {
	try {
		await fetchPluginCatalog()
		return pluginUpdates.value
	} catch {
		return []
	}
}

export async function updateAllPlugins(): Promise<InstalledPlugin[]> {
	const targets = [...pluginUpdates.value]
	const updated: InstalledPlugin[] = []
	for (const entry of targets) {
		updated.push(await installPluginRemote(entry.download_url, entry.sha256))
	}
	return updated
}

export function invokePluginAction(
	pluginId: string,
	actionId: string,
	context: Record<string, unknown> = {},
): void {
	const entry = activeWorkers.get(pluginId)
	if (!entry) return
	entry.worker.postMessage({ type: 'action:invoke', actionId, context })
}

export async function searchPluginContentSource(
	pluginId: string,
	params: { query: string; projectType: string; offset: number; limit: number; sort?: string },
): Promise<NoctrinthContentSourceSearchResult> {
	const entry = activeWorkers.get(pluginId)
	if (!entry) {
		throw new Error(`Plugin "${pluginId}" is not running`)
	}
	return await new Promise<NoctrinthContentSourceSearchResult>((resolve, reject) => {
		const callId = nextHostCallId++
		const timer = setTimeout(() => {
			if (entry.pendingHostCalls.delete(callId)) {
				reject(new Error(`Content source "${pluginId}" timed out`))
			}
		}, 15_000)
		entry.pendingHostCalls.set(callId, {
			resolve: (v) => {
				clearTimeout(timer)
				resolve(v as NoctrinthContentSourceSearchResult)
			},
			reject: (e) => {
				clearTimeout(timer)
				reject(e)
			},
		})
		entry.worker.postMessage({ type: 'contentSource:search', callId, params })
	})
}

function removePluginStyles(id: string) {
	document.querySelectorAll(`style[data-noctrinth-plugin="${CSS.escape(id)}"]`).forEach((el) => {
		el.remove()
	})
}

function resolveRelativeAssetPath(baseFile: string, rawRef: string): string | null {
	const cleaned = rawRef.split('#')[0].split('?')[0].trim()
	if (!cleaned) return null
	const baseParts = baseFile.replace(/\\/g, '/').split('/')
	baseParts.pop()
	const refParts = cleaned.replace(/\\/g, '/').split('/')
	const combined = cleaned.startsWith('/') ? refParts.slice(1) : [...baseParts, ...refParts]
	const out: string[] = []
	for (const part of combined) {
		if (!part || part === '.') continue
		if (part === '..') {
			if (out.length === 0) return null
			out.pop()
		} else {
			out.push(part)
		}
	}
	return out.length > 0 ? out.join('/') : null
}

async function inlineCssLocalAssets(
	pluginId: string,
	cssFile: string,
	css: string,
	cache: Map<string, string>,
): Promise<string> {
	const urlRegex = /url\(\s*(['"]?)([^)'"]+)\1\s*\)/gi
	const matches = [...css.matchAll(urlRegex)]
	if (matches.length === 0) return css

	const replacements = new Map<string, string>()
	for (const match of matches) {
		const rawUrl = match[2].trim()
		if (
			!rawUrl ||
			rawUrl.startsWith('data:') ||
			rawUrl.startsWith('http://') ||
			rawUrl.startsWith('https://') ||
			rawUrl.startsWith('blob:') ||
			rawUrl.startsWith('#')
		) {
			continue
		}
		const resolved = resolveRelativeAssetPath(cssFile, rawUrl)
		if (!resolved) continue
		let dataUri = cache.get(resolved)
		if (!dataUri) {
			try {
				const asset = await invoke<{ mime: string; base64: string }>(
					'plugin:noctrinth-plugins|plugins_read_asset_base64',
					{ id: pluginId, relPath: resolved },
				)
				dataUri = `data:${asset.mime};base64,${asset.base64}`
				cache.set(resolved, dataUri)
			} catch {
				continue
			}
		}
		replacements.set(match[0], `url("${dataUri}")`)
	}

	if (replacements.size === 0) return css
	let result = css
	for (const [original, next] of replacements) {
		result = result.split(original).join(next)
	}
	return result
}

async function applyPluginStyles(plugin: InstalledPlugin) {
	const id = plugin.manifest.id
	removePluginStyles(id)
	const styles = plugin.manifest.styles ?? []
	const assetCache = new Map<string, string>()
	for (const relPath of styles) {
		const rawCss = await invoke<string>('plugin:noctrinth-plugins|plugins_read_asset', {
			id,
			relPath,
		})
		const css = await inlineCssLocalAssets(id, relPath, rawCss, assetCache)
		const styleEl = document.createElement('style')
		styleEl.setAttribute('data-noctrinth-plugin', id)
		styleEl.setAttribute('data-noctrinth-plugin-file', relPath)
		styleEl.textContent = css
		document.head.appendChild(styleEl)
	}
}

function buildWorkerBootstrap(plugin: InstalledPlugin, userCode: string): string {
	const meta = JSON.stringify({
		id: plugin.manifest.id,
		version: plugin.manifest.version,
	})
	return `
const __meta = ${meta};
const __pending = new Map();
const __listeners = new Map();
const __actions = new Map();
const __panelListeners = [];
const __streams = new Map();
let __contentSourceSearch = null;
let __reqId = 1;

function __formatArgs(args) {
	return args.map((a) => {
		if (typeof a === 'string') return a;
		try { return JSON.stringify(a); } catch { return String(a); }
	}).join(' ');
}

for (const [fnName, level] of [['log', 'info'], ['info', 'info'], ['warn', 'warn'], ['error', 'error']]) {
	const orig = console[fnName];
	console[fnName] = (...args) => {
		self.postMessage({ type: 'log', level, message: __formatArgs(args) });
		if (orig) orig.apply(console, args);
	};
}

function __rpc(method, payload) {
	return new Promise((resolve, reject) => {
		const id = __reqId++;
		__pending.set(id, { resolve, reject });
		self.postMessage({ type: 'rpc', id, method, payload });
	});
}

self.addEventListener('message', async (event) => {
	const msg = event.data;
	if (!msg) return;
	if (msg.type === 'rpc:result' && __pending.has(msg.id)) {
		const p = __pending.get(msg.id);
		__pending.delete(msg.id);
		if (msg.error) p.reject(new Error(msg.error));
		else p.resolve(msg.result);
	} else if (msg.type === 'event') {
		const handlers = __listeners.get(msg.event);
		if (handlers) {
			for (const fn of handlers) {
				try { await fn(msg.payload); } catch (e) { console.error('[Noctrinth Plugin]', __meta.id, e); }
			}
		}
	} else if (msg.type === 'action:invoke') {
		const fn = __actions.get(msg.actionId);
		if (fn) {
			try { await fn(msg.context || {}); } catch (e) {
				self.postMessage({ type: 'error', error: String(e && e.message ? e.message : e) });
			}
		}
	} else if (msg.type === 'stream:event') {
		const s = __streams.get(msg.streamId);
		if (s) {
			if (typeof msg.chunk === 'string' && msg.chunk.length > 0 && s.onChunk) {
				try { await s.onChunk(msg.chunk); } catch (e) { console.error('[Noctrinth Plugin]', __meta.id, e); }
			}
			if (msg.done) {
				s.done = true;
				s.error = msg.error || null;
				if (s.headReady) {
					__streams.delete(msg.streamId);
					if (s.error) s.reject(new Error(s.error));
					else s.resolve(s.head);
				}
			}
		}
	} else if (msg.type === 'panel:message') {
		for (const fn of __panelListeners) {
			try { await fn(msg.data, msg.panelId); } catch (e) {
				self.postMessage({ type: 'error', error: String(e && e.message ? e.message : e) });
			}
		}
	} else if (msg.type === 'contentSource:search') {
		try {
			if (!__contentSourceSearch) throw new Error('No content source registered');
			const result = await __contentSourceSearch(msg.params || {});
			self.postMessage({ type: 'host:result', callId: msg.callId, result });
		} catch (e) {
			self.postMessage({ type: 'host:result', callId: msg.callId, error: String(e && e.message ? e.message : e) });
		}
	}
});

self.noctrinth = Object.freeze({
	id: __meta.id,
	version: __meta.version,
	fetch: (url, init) => __rpc('fetch', { url, init }),
	fetchStream: (url, init, onChunk) => {
		const streamId = 'stream_' + (__reqId++);
		return new Promise((resolve, reject) => {
			const entry = { onChunk, resolve, reject, head: null, headReady: false, done: false, error: null };
			__streams.set(streamId, entry);
			__rpc('fetchStream', { streamId, url, init })
				.then((head) => {
					entry.head = head;
					entry.headReady = true;
					if (entry.done) {
						__streams.delete(streamId);
						if (entry.error) reject(new Error(entry.error));
						else resolve(head);
					}
				})
				.catch((err) => {
					__streams.delete(streamId);
					reject(err);
				});
		});
	},
	storage: Object.freeze({
		get: (key) => __rpc('storage:get', { key }),
		set: (key, value) => __rpc('storage:set', { key, value }),
		remove: (key) => __rpc('storage:remove', { key }),
	}),
	settings: Object.freeze({
		get: (key) => __rpc('settings:get', { key }),
		getAll: () => __rpc('settings:getAll', {}),
	}),
	notify: (opts) => __rpc('notify', opts),
	clipboard: Object.freeze({
		writeText: (text) => __rpc('clipboard:writeText', { text }),
	}),
	app: Object.freeze({
		getInfo: () => __rpc('app:getInfo', {}),
	}),
	ui: Object.freeze({
		registerAction: async (action, handler) => {
			__actions.set(action.id, handler);
			await __rpc('ui:registerAction', action);
		},
		showModal: (opts) => __rpc('ui:showModal', opts),
		showForm: (opts) => __rpc('ui:showForm', opts),
		updateModal: (opts) => __rpc('ui:updateModal', opts),
		openPanel: (opts) => __rpc('ui:openPanel', opts),
		closePanel: () => __rpc('ui:closePanel', {}),
		postPanelMessage: (data, targetId) => __rpc('ui:postPanelMessage', { data, targetId }),
		onPanelMessage: (handler) => {
			if (typeof handler === 'function') __panelListeners.push(handler);
		},
		registerPage: (page) => __rpc('ui:registerPage', page),
	}),
	instances: Object.freeze({
		list: () => __rpc('instances:list', {}),
		get: (instanceId) => __rpc('instances:get', { instanceId }),
		getContent: (instanceId) => __rpc('instances:getContent', { instanceId }),
		launch: (instanceId) => __rpc('instances:launch', { instanceId }),
		removeContent: (instanceId, filePath) => __rpc('instances:removeContent', { instanceId, filePath }),
		toggleContent: (instanceId, filePath, enabled) => __rpc('instances:toggleContent', { instanceId, filePath, enabled }),
		getLogs: (instanceId) => __rpc('instances:getLogs', { instanceId }),
		listFiles: (instanceId, relDir) => __rpc('instances:listFiles', { instanceId, relDir }),
		readFile: (instanceId, relPath) => __rpc('instances:readFile', { instanceId, relPath }),
		writeFile: (instanceId, relPath, content) => __rpc('instances:writeFile', { instanceId, relPath, content }),
	}),
	contentSource: Object.freeze({
		register: async (def) => {
			__contentSourceSearch = def.search;
			await __rpc('contentSource:register', {
				label: def.label,
				projectTypes: def.projectTypes || ['mod', 'modpack', 'resourcepack', 'shader', 'datapack'],
			});
		},
	}),
	on: (event, handler) => {
		if (!__listeners.has(event)) __listeners.set(event, []);
		__listeners.get(event).push(handler);
	},
	log: (...args) => self.postMessage({ type: 'log', level: 'info', message: __formatArgs(args) }),
});

(async () => {
${userCode}
})().catch((err) => {
	self.postMessage({ type: 'error', error: String(err && err.message ? err.message : err) });
});
`
}

async function startPluginWorker(plugin: InstalledPlugin) {
	const id = plugin.manifest.id
	stopPluginWorker(id)
	const entry = plugin.manifest.entry
	if (!entry) return

	const userCode = await invoke<string>('plugin:noctrinth-plugins|plugins_read_asset', {
		id,
		relPath: entry,
	})
	const blob = new Blob([buildWorkerBootstrap(plugin, userCode)], {
		type: 'application/javascript',
	})
	const blobUrl = URL.createObjectURL(blob)
	const worker = new Worker(blobUrl)
	const storagePrefix = `noctrinth:plugin-storage:${id}:`
	const allowedEvents = new Set<string>(plugin.manifest.permissions?.events ?? [])
	const allowedInstances = new Set<string>(plugin.manifest.permissions?.instances ?? [])
	const allowedUi = new Set<string>(plugin.manifest.permissions?.ui ?? [])
	const pendingHostCalls = new Map<
		number,
		{ resolve: (v: unknown) => void; reject: (e: Error) => void }
	>()

	const requireInstancePerm = (perm: 'read' | 'write' | 'launch') => {
		if (!allowedInstances.has(perm)) {
			throw new Error(`Plugin "${id}" lacks permissions.instances.${perm}`)
		}
	}
	const requireUiPerm = (
		perm: 'notifications' | 'actions' | 'modal' | 'panel' | 'page' | 'clipboard',
	) => {
		if (!allowedUi.has(perm)) {
			throw new Error(`Plugin "${id}" lacks permissions.ui.${perm}`)
		}
	}

	const resolvePanelHtml = async (opts: { entry?: string; html?: string }): Promise<string> => {
		if (opts.entry) {
			return await invoke<string>('plugin:noctrinth-plugins|plugins_read_asset', {
				id,
				relPath: String(opts.entry),
			})
		}
		if (typeof opts.html === 'string') {
			return opts.html
		}
		throw new Error('Panel or page must specify either "entry" or "html"')
	}

	let errorCount = 0
	let rpcWindowStart = Date.now()
	let rpcCount = 0
	let logWindowStart = Date.now()
	let logCount = 0
	const notifyTimestamps: number[] = []

	const checkRateLimit = (
		now: number,
		start: number,
		count: number,
		windowMs: number,
	): { start: number; count: number } => {
		if (now - start >= windowMs) return { start: now, count: 1 }
		return { start, count: count + 1 }
	}

	worker.onmessage = async (event) => {
		const msg = event.data
		if (!msg) return
		const now = Date.now()
		if (msg.type === 'log') {
			const next = checkRateLimit(now, logWindowStart, logCount, LOG_WINDOW_MS)
			logWindowStart = next.start
			logCount = next.count
			if (logCount > MAX_LOGS_PER_WINDOW) return
			const level: PluginLogEntry['level'] =
				msg.level === 'warn' || msg.level === 'error' ? msg.level : 'info'
			const message = String(msg.message ?? (Array.isArray(msg.args) ? msg.args.join(' ') : ''))
			appendPluginLog(id, level, message)
			return
		}
		if (msg.type === 'error') {
			const text = String(msg.error)
			appendPluginLog(id, 'error', text)
			pluginRuntimeErrors.value = { ...pluginRuntimeErrors.value, [id]: text }
			errorCount++
			if (errorCount >= MAX_ERRORS_BEFORE_DISABLE) {
				void tripPluginCircuitBreaker(plugin, text)
			}
			return
		}
		if (msg.type === 'host:result' && pendingHostCalls.has(msg.callId)) {
			const p = pendingHostCalls.get(msg.callId)!
			pendingHostCalls.delete(msg.callId)
			if (msg.error) p.reject(new Error(String(msg.error)))
			else p.resolve(msg.result)
			return
		}
		if (msg.type === 'rpc') {
			const nextRpc = checkRateLimit(now, rpcWindowStart, rpcCount, RPC_WINDOW_MS)
			rpcWindowStart = nextRpc.start
			rpcCount = nextRpc.count
			if (rpcCount > FLOOD_KILL_RPC_PER_WINDOW) {
				void tripPluginCircuitBreaker(plugin, 'Stopped automatically: excessive RPC flood detected')
				return
			}
			if (rpcCount > MAX_RPC_PER_WINDOW) {
				worker.postMessage({
					type: 'rpc:result',
					id: msg.id,
					error: 'Rate limit exceeded (too many RPC calls per second)',
				})
				return
			}
			try {
				let result: unknown = null
				switch (msg.method) {
					case 'fetch':
						result = await invoke('plugin:noctrinth-plugins|plugins_http_fetch', {
							id,
							url: msg.payload?.url,
							method: msg.payload?.init?.method ?? null,
							headers: msg.payload?.init?.headers ?? null,
							body: msg.payload?.init?.body ?? null,
						})
						break
					case 'fetchStream':
						result = await invoke('plugin:noctrinth-plugins|plugins_http_fetch_stream', {
							id,
							streamId: String(msg.payload?.streamId ?? ''),
							url: msg.payload?.url,
							method: msg.payload?.init?.method ?? null,
							headers: msg.payload?.init?.headers ?? null,
							body: msg.payload?.init?.body ?? null,
						})
						break
					case 'storage:get':
						result = localStorage.getItem(storagePrefix + String(msg.payload?.key ?? ''))
						break
					case 'storage:set':
						localStorage.setItem(
							storagePrefix + String(msg.payload?.key ?? ''),
							String(msg.payload?.value ?? ''),
						)
						break
					case 'storage:remove':
						localStorage.removeItem(storagePrefix + String(msg.payload?.key ?? ''))
						break
					case 'settings:get': {
						const all = getPluginSettings(plugin)
						result = all[String(msg.payload?.key ?? '')] ?? null
						break
					}
					case 'settings:getAll':
						result = getPluginSettings(plugin)
						break
					case 'notify': {
						requireUiPerm('notifications')
						const cutoff = Date.now() - NOTIFY_WINDOW_MS
						while (notifyTimestamps.length > 0 && notifyTimestamps[0] < cutoff) {
							notifyTimestamps.shift()
						}
						if (notifyTimestamps.length >= MAX_NOTIFY_PER_WINDOW) {
							appendPluginLog(id, 'warn', 'Notification throttled (rate limit)')
							break
						}
						notifyTimestamps.push(Date.now())
						notificationHandler?.({
							title: String(msg.payload?.title ?? plugin.manifest.name),
							text: msg.payload?.text ? String(msg.payload.text) : undefined,
							type: msg.payload?.type ?? 'info',
						})
						break
					}
					case 'ui:registerAction': {
						requireUiPerm('actions')
						const slot = msg.payload?.slot as NoctrinthActionSlot
						if (!['instance-header', 'sidebar', 'mod-menu'].includes(slot)) {
							throw new Error(`Invalid UI slot "${slot}"`)
						}
						const actionId = String(msg.payload?.id ?? '')
						const next = pluginActions.value.filter(
							(a) => !(a.pluginId === id && a.actionId === actionId),
						)
						next.push({
							pluginId: id,
							pluginName: plugin.manifest.name,
							actionId,
							slot,
							label: String(msg.payload?.label ?? actionId),
							tooltip: msg.payload?.tooltip ? String(msg.payload.tooltip) : undefined,
						})
						pluginActions.value = next
						break
					}
					case 'clipboard:writeText': {
						requireUiPerm('clipboard')
						const text = String(msg.payload?.text ?? '')
						if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
							await navigator.clipboard.writeText(text)
						}
						break
					}
					case 'app:getInfo': {
						const docEl = typeof document !== 'undefined' ? document.documentElement : null
						const themeClass =
							docEl?.className.split(/\s+/).find((c) => c.endsWith('-mode')) ?? 'dark-mode'
						result = {
							locale: docEl?.lang || (typeof navigator !== 'undefined' ? navigator.language : 'en'),
							theme: themeClass,
							safeMode: pluginsSafeMode.value,
						}
						break
					}
					case 'ui:showModal':
					case 'ui:showForm': {
						requireUiPerm('modal')
						const payload = (msg.payload ?? {}) as NoctrinthPluginModalRequest
						const mode = msg.method === 'ui:showForm' ? 'form' : 'modal'
						if (activePluginModal.value) {
							resolvePluginModal(null)
						}
						result = await new Promise((resolve) => {
							activePluginModal.value = {
								pluginId: id,
								pluginName: plugin.manifest.name,
								mode,
								request: {
									title: String(payload.title ?? plugin.manifest.name),
									description: payload.description ? String(payload.description) : undefined,
									blocks: Array.isArray(payload.blocks) ? payload.blocks : [],
									actions: Array.isArray(payload.actions) ? payload.actions : [],
								},
								resolve,
							}
						})
						break
					}
					case 'ui:updateModal': {
						requireUiPerm('modal')
						const current = activePluginModal.value
						if (current && current.pluginId === id) {
							const patch = (msg.payload ?? {}) as Partial<NoctrinthPluginModalRequest>
							activePluginModal.value = {
								...current,
								request: {
									title: patch.title !== undefined ? String(patch.title) : current.request.title,
									description:
										patch.description !== undefined
											? String(patch.description)
											: current.request.description,
									blocks: Array.isArray(patch.blocks) ? patch.blocks : current.request.blocks,
									actions: Array.isArray(patch.actions) ? patch.actions : current.request.actions,
								},
							}
						}
						break
					}
					case 'ui:openPanel': {
						requireUiPerm('panel')
						const payload = (msg.payload ?? {}) as NoctrinthPluginPanelRequest
						const html = await resolvePanelHtml(payload)
						const panelId = String(payload.id || 'default')
						const rawWidth = Number(payload.width ?? 460)
						const width = Number.isFinite(rawWidth) ? Math.min(Math.max(rawWidth, 320), 900) : 460
						activePluginPanel.value = {
							pluginId: id,
							pluginName: plugin.manifest.name,
							panelId,
							title: String(payload.title ?? plugin.manifest.name),
							html,
							width,
						}
						break
					}
					case 'ui:closePanel': {
						requireUiPerm('panel')
						if (activePluginPanel.value?.pluginId === id) {
							activePluginPanel.value = null
						}
						break
					}
					case 'ui:postPanelMessage': {
						const targetId = String(
							msg.payload?.targetId ?? activePluginPanel.value?.panelId ?? 'default',
						)
						const exact = panelMessageSinks.get(`${id}:${targetId}`)
						if (exact) {
							exact(msg.payload?.data)
						} else {
							for (const [key, sink] of panelMessageSinks.entries()) {
								if (key.startsWith(`${id}:`)) {
									sink(msg.payload?.data)
								}
							}
						}
						break
					}
					case 'ui:registerPage': {
						requireUiPerm('page')
						const payload = (msg.payload ?? {}) as NoctrinthPluginPageDeclaration
						const pageId = sanitizeToken(String(payload.id || 'main'))
						const html = await resolvePanelHtml(payload)
						const next = pluginPages.value.filter(
							(p) => !(p.pluginId === id && p.pageId === pageId),
						)
						next.push({
							pluginId: id,
							pluginName: plugin.manifest.name,
							pageId,
							title: String(payload.title ?? plugin.manifest.name),
							tooltip: payload.tooltip ? String(payload.tooltip) : undefined,
							html,
						})
						pluginPages.value = next
						break
					}
					case 'contentSource:register': {
						if (plugin.manifest.type !== 'content-source') {
							throw new Error('Only content-source plugins can register a content source')
						}
						const next = pluginContentSources.value.filter((s) => s.pluginId !== id)
						next.push({
							pluginId: id,
							label: String(msg.payload?.label ?? plugin.manifest.name),
							projectTypes: Array.isArray(msg.payload?.projectTypes)
								? msg.payload.projectTypes.map(String)
								: ['mod', 'modpack', 'resourcepack', 'shader', 'datapack'],
						})
						pluginContentSources.value = next
						break
					}
					case 'instances:list':
						requireInstancePerm('read')
						result = await listInstances()
						break
					case 'instances:get':
						requireInstancePerm('read')
						result = await getInstance(String(msg.payload?.instanceId ?? ''))
						break
					case 'instances:getContent':
						requireInstancePerm('read')
						result = await get_content_items(String(msg.payload?.instanceId ?? ''))
						break
					case 'instances:getLogs':
						requireInstancePerm('read')
						result = await get_logs(String(msg.payload?.instanceId ?? ''), false)
						break
					case 'instances:launch':
						requireInstancePerm('launch')
						await runInstance(String(msg.payload?.instanceId ?? ''))
						break
					case 'instances:removeContent':
						requireInstancePerm('write')
						await remove_project(
							String(msg.payload?.instanceId ?? ''),
							String(msg.payload?.filePath ?? ''),
						)
						break
					case 'instances:toggleContent':
						requireInstancePerm('write')
						await toggle_disable_project(
							String(msg.payload?.instanceId ?? ''),
							String(msg.payload?.filePath ?? ''),
							typeof msg.payload?.enabled === 'boolean' ? msg.payload.enabled : undefined,
						)
						break
					case 'instances:listFiles':
						requireInstancePerm('read')
						result = await invoke('plugin:noctrinth-plugins|plugins_instance_list_files', {
							id,
							instanceId: String(msg.payload?.instanceId ?? ''),
							relDir: msg.payload?.relDir ? String(msg.payload.relDir) : null,
						})
						break
					case 'instances:readFile':
						requireInstancePerm('read')
						result = await invoke('plugin:noctrinth-plugins|plugins_instance_read_file', {
							id,
							instanceId: String(msg.payload?.instanceId ?? ''),
							relPath: String(msg.payload?.relPath ?? ''),
						})
						break
					case 'instances:writeFile':
						requireInstancePerm('write')
						await invoke('plugin:noctrinth-plugins|plugins_instance_write_file', {
							id,
							instanceId: String(msg.payload?.instanceId ?? ''),
							relPath: String(msg.payload?.relPath ?? ''),
							content: String(msg.payload?.content ?? ''),
						})
						break
					default:
						throw new Error(`Unknown RPC method: ${msg.method}`)
				}
				worker.postMessage({ type: 'rpc:result', id: msg.id, result })
			} catch (err) {
				worker.postMessage({
					type: 'rpc:result',
					id: msg.id,
					error: err instanceof Error ? err.message : String(err),
				})
			}
		}
	}

	worker.onerror = (err) => {
		const message = err.message || 'Worker execution error'
		void tripPluginCircuitBreaker(plugin, message)
	}

	activeWorkers.set(id, { worker, blobUrl, events: allowedEvents, pendingHostCalls })
}

async function tripPluginCircuitBreaker(plugin: InstalledPlugin, reason: string): Promise<void> {
	const id = plugin.manifest.id
	deactivatePlugin(id)
	appendPluginLog(id, 'error', `Auto-disabled: ${reason}`)
	pluginRuntimeErrors.value = {
		...pluginRuntimeErrors.value,
		[id]: `Auto-disabled after error: ${reason}`,
	}
	try {
		await invoke('plugin:noctrinth-plugins|plugins_set_enabled', { id, enabled: false })
		await listInstalledPlugins()
	} catch {
		// Best-effort persist
	}
	notificationHandler?.({
		title: `Plugin "${plugin.manifest.name}" disabled`,
		text: reason,
		type: 'error',
	})
}

function stopPluginWorker(id: string) {
	const entry = activeWorkers.get(id)
	if (entry) {
		for (const p of entry.pendingHostCalls.values()) {
			p.reject(new Error(`Plugin "${id}" stopped`))
		}
		entry.worker.terminate()
		URL.revokeObjectURL(entry.blobUrl)
		activeWorkers.delete(id)
	}
	if (activePluginModal.value?.pluginId === id) {
		resolvePluginModal(null)
	}
	if (activePluginPanel.value?.pluginId === id) {
		activePluginPanel.value = null
	}
	pluginActions.value = pluginActions.value.filter((a) => a.pluginId !== id)
	pluginContentSources.value = pluginContentSources.value.filter((s) => s.pluginId !== id)
	pluginPages.value = pluginPages.value.filter((p) => p.pluginId !== id)
}

async function registerManifestPages(plugin: InstalledPlugin): Promise<void> {
	const pages = plugin.manifest.pages ?? []
	if (pages.length === 0) return
	if (!(plugin.manifest.permissions?.ui ?? []).includes('page')) return
	const id = plugin.manifest.id
	for (const decl of pages) {
		const pageId = sanitizeToken(String(decl.id || 'main'))
		let html = ''
		if (decl.entry) {
			html = await invoke<string>('plugin:noctrinth-plugins|plugins_read_asset', {
				id,
				relPath: String(decl.entry),
			})
		} else if (typeof decl.html === 'string') {
			html = decl.html
		} else {
			continue
		}
		const next = pluginPages.value.filter((p) => !(p.pluginId === id && p.pageId === pageId))
		next.push({
			pluginId: id,
			pluginName: plugin.manifest.name,
			pageId,
			title: String(decl.title || plugin.manifest.name),
			tooltip: decl.tooltip ? String(decl.tooltip) : undefined,
			html,
		})
		pluginPages.value = next
	}
}

export async function activatePlugin(plugin: InstalledPlugin): Promise<void> {
	if (pluginsSafeMode.value) return
	const id = plugin.manifest.id
	const { [id]: _removed, ...rest } = pluginRuntimeErrors.value
	pluginRuntimeErrors.value = rest

	try {
		applyPluginSettingVars(plugin)
		await applyPluginStyles(plugin)
		await registerManifestPages(plugin)
		await startPluginWorker(plugin)
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		await tripPluginCircuitBreaker(plugin, message)
	}
}

export function deactivatePlugin(id: string): void {
	removePluginSettingVars(id)
	removePluginStyles(id)
	stopPluginWorker(id)
}

export function enablePluginsSafeMode(notify = true): void {
	pluginsSafeMode.value = true
	quickCssEnabled.value = false
	applyQuickCss()
	for (const plugin of installedPlugins.value) {
		deactivatePlugin(plugin.manifest.id)
	}
	for (const id of [...activeWorkers.keys()]) {
		deactivatePlugin(id)
	}
	if (notify) {
		notificationHandler?.({
			title: 'Plugins Safe Mode enabled',
			text: 'All plugins and Quick CSS have been temporarily disabled.',
			type: 'warning',
		})
	}
}

export async function exitPluginsSafeMode(): Promise<void> {
	pluginsSafeMode.value = false
	applyQuickCss()
	const list = await listInstalledPlugins()
	await Promise.all(list.filter((p) => p.enabled).map((p) => activatePlugin(p)))
}

export function emitPluginEvent(
	event: NoctrinthPluginEvent,
	payload: Record<string, unknown>,
): void {
	for (const entry of activeWorkers.values()) {
		if (entry.events.has(event)) {
			entry.worker.postMessage({ type: 'event', event, payload })
		}
	}
}

let initialized = false

export async function initNoctrinthPlugins(): Promise<void> {
	if (initialized) return
	initialized = true

	if (typeof window !== 'undefined') {
		window.addEventListener('keydown', (event) => {
			if (event.ctrlKey && event.shiftKey && event.altKey && event.code === 'KeyP') {
				event.preventDefault()
				enablePluginsSafeMode(true)
			}
		})
		void listen<{
			pluginId: string
			streamId: string
			chunk?: string | null
			done: boolean
			error?: string | null
		}>('noctrinth-plugin-stream', (event) => {
			const payload = event.payload
			if (!payload) return
			const entry = activeWorkers.get(payload.pluginId)
			if (!entry) return
			entry.worker.postMessage({
				type: 'stream:event',
				streamId: payload.streamId,
				chunk: payload.chunk ?? null,
				done: Boolean(payload.done),
				error: payload.error ?? null,
			})
		})
	}

	try {
		const safeFlag = await invoke<boolean>('plugin:noctrinth-plugins|plugins_is_safe_mode').catch(
			() => false,
		)
		const list = await listInstalledPlugins()
		if (safeFlag) {
			enablePluginsSafeMode(true)
			return
		}
		applyQuickCss()
		await Promise.all(list.filter((p) => p.enabled).map((p) => activatePlugin(p)))
		if (list.length > 0) {
			void checkPluginUpdates()
		}
	} catch (error) {
		console.warn('Could not initialize Noctrinth plugins:', error)
	}
}
