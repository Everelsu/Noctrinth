import type {
	NoctrinthActionSlot,
	NoctrinthPluginManifest,
	NoctrinthPluginModalRequest,
	NoctrinthPluginType,
} from './sdk'

export type {
	NoctrinthActionSlot,
	NoctrinthContentSourceHit,
	NoctrinthContentSourceSearchResult,
	NoctrinthPluginEvent,
	NoctrinthPluginManifest,
	NoctrinthPluginModalAction,
	NoctrinthPluginModalBlock,
	NoctrinthPluginModalRequest,
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

export interface PluginLogEntry {
	timestamp: number
	level: 'info' | 'warn' | 'error'
	message: string
}

export interface ActivePluginModal {
	pluginId: string
	pluginName: string
	request: NoctrinthPluginModalRequest
	resolve: (actionId: string | null) => void
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
