import type {
	NoctrinthActionSlot,
	NoctrinthPluginFormResult,
	NoctrinthPluginManifest,
	NoctrinthPluginModalRequest,
	NoctrinthPluginPageDeclaration,
	NoctrinthPluginPanelRequest,
	NoctrinthPluginPreset,
	NoctrinthPluginType,
} from './sdk'

export type {
	NoctrinthActionSlot,
	NoctrinthContentSourceHit,
	NoctrinthContentSourceSearchResult,
	NoctrinthPluginAppInfo,
	NoctrinthPluginEvent,
	NoctrinthPluginFormResult,
	NoctrinthPluginInstanceFileEntry,
	NoctrinthPluginManifest,
	NoctrinthPluginModalAction,
	NoctrinthPluginModalBlock,
	NoctrinthPluginModalRequest,
	NoctrinthPluginPageDeclaration,
	NoctrinthPluginPanelRequest,
	NoctrinthPluginPreset,
	NoctrinthPluginSettingCondition,
	NoctrinthPluginSettingField,
	NoctrinthPluginType,
} from './sdk'

export interface InstalledPlugin {
	manifest: NoctrinthPluginManifest
	enabled: boolean
	source: 'local' | 'catalog'
	sha256: string
	installed_at: number
}

export interface RegisteredPluginAction {
	pluginId: string
	pluginName: string
	actionId: string
	slot: NoctrinthActionSlot
	label: string
	tooltip?: string
}

export interface RegisteredPluginContentSource {
	pluginId: string
	label: string
	projectTypes: string[]
}

export interface RegisteredPluginPage {
	pluginId: string
	pluginName: string
	pageId: string
	title: string
	tooltip?: string
	html: string
}

export interface ActivePluginPanel {
	pluginId: string
	pluginName: string
	panelId: string
	title: string
	html: string
	width: number
}

export interface PluginLogEntry {
	timestamp: number
	level: 'info' | 'warn' | 'error'
	message: string
}

export interface ActivePluginModal {
	pluginId: string
	pluginName: string
	mode: 'modal' | 'form'
	request: NoctrinthPluginModalRequest
	resolve: (result: string | null | NoctrinthPluginFormResult) => void
}

export interface CatalogPluginEntry {
	id: string
	name: string
	version: string
	apiVersion: number
	type: NoctrinthPluginType
	author?: string
	description?: string
	homepage?: string
	repository?: string
	icon_url?: string
	banner_url?: string
	tags?: string[]
	download_url: string
	sha256: string
	commit?: string
	permissions?: {
		domains?: string[]
		events?: string[]
		instances?: string[]
		ui?: string[]
	}
}

export interface PluginCatalog {
	schema_version?: number
	updated_at?: string
	plugins: CatalogPluginEntry[]
}

export type { NoctrinthPluginPageDeclaration, NoctrinthPluginPanelRequest, NoctrinthPluginPreset }
