export type NoctrinthPluginType = 'theme' | 'content-source' | 'extension'

export type NoctrinthPluginEvent =
	| 'instance:launch'
	| 'instance:exit'
	| 'instance:crash'
	| 'download:complete'
	| 'settings:change'

export type NoctrinthInstancePermission = 'read' | 'write' | 'launch'
export type NoctrinthUiPermission = 'notifications' | 'actions' | 'settings' | 'modal'
export type NoctrinthActionSlot = 'instance-header' | 'sidebar' | 'mod-menu'

export interface NoctrinthPluginSettingField {
	key: string
	type: 'string' | 'boolean' | 'number' | 'select' | 'color'
	label: string
	description?: string
	placeholder?: string
	default?: string | boolean | number
	min?: number
	max?: number
	cssVar?: string
	options?: Array<{ value: string; label: string }>
}

export type NoctrinthPluginModalBlock =
	| { type: 'text'; text: string }
	| { type: 'code'; text: string }
	| { type: 'progress'; value: number; max?: number; label?: string }
	| { type: 'list'; items: Array<{ title: string; subtitle?: string }> }

export interface NoctrinthPluginModalAction {
	id: string
	label: string
	color?: 'brand' | 'red' | 'standard'
}

export interface NoctrinthPluginModalRequest {
	title: string
	description?: string
	blocks?: NoctrinthPluginModalBlock[]
	actions?: NoctrinthPluginModalAction[]
}

export interface NoctrinthPluginPermissions {
	domains?: string[]
	events?: Exclude<NoctrinthPluginEvent, 'settings:change'>[]
	instances?: NoctrinthInstancePermission[]
	ui?: NoctrinthUiPermission[]
}

export interface NoctrinthPluginManifest {
	$schema?: string
	id: string
	name: string
	version: string
	apiVersion: 1
	type: NoctrinthPluginType
	author?: string
	description?: string
	homepage?: string
	icon?: string
	styles?: string[]
	entry?: string
	permissions?: NoctrinthPluginPermissions
	settings?: NoctrinthPluginSettingField[]
}

export interface NoctrinthPluginFetchResponse {
	status: number
	ok: boolean
	headers: Record<string, string>
	body: string
}

export interface NoctrinthContentSourceHit {
	id: string
	slug?: string
	title: string
	description?: string
	icon_url?: string
	author?: string
	downloads?: number
	follows?: number
	categories?: string[]
	loaders?: string[]
	project_type?: string
	external_url?: string
}

export interface NoctrinthContentSourceSearchResult {
	hits: NoctrinthContentSourceHit[]
	total_hits: number
	offset: number
	limit: number
}

export interface NoctrinthPluginSdk {
	readonly id: string
	readonly version: string
	fetch(
		url: string,
		init?: { method?: string; headers?: Record<string, string>; body?: string },
	): Promise<NoctrinthPluginFetchResponse>
	storage: {
		get(key: string): Promise<string | null>
		set(key: string, value: string): Promise<void>
		remove(key: string): Promise<void>
	}
	settings: {
		get<T = string | boolean | number | null>(key: string): Promise<T>
		getAll(): Promise<Record<string, unknown>>
	}
	notify(opts: {
		title: string
		text?: string
		type?: 'info' | 'success' | 'warning' | 'error'
	}): Promise<void>
	ui: {
		registerAction(
			action: { id: string; slot: NoctrinthActionSlot; label: string; tooltip?: string },
			handler: (context: Record<string, unknown>) => void | Promise<void>,
		): Promise<void>
		showModal(opts: NoctrinthPluginModalRequest): Promise<string | null>
	}
	instances: {
		list(): Promise<Array<Record<string, unknown>>>
		get(instanceId: string): Promise<Record<string, unknown> | null>
		getContent(instanceId: string): Promise<Array<Record<string, unknown>>>
		launch(instanceId: string): Promise<void>
		removeContent(instanceId: string, filePath: string): Promise<void>
		toggleContent(instanceId: string, filePath: string, enabled?: boolean): Promise<void>
		getLogs(instanceId: string): Promise<Array<Record<string, unknown>>>
	}
	contentSource: {
		register(def: {
			label: string
			projectTypes?: string[]
			search: (params: {
				query: string
				projectType: string
				offset: number
				limit: number
				sort?: string
			}) => Promise<NoctrinthContentSourceSearchResult>
		}): Promise<void>
	}
	on(event: NoctrinthPluginEvent, handler: (payload: Record<string, unknown>) => void): void
	log(...args: unknown[]): void
}

declare global {
	const noctrinth: NoctrinthPluginSdk
}
