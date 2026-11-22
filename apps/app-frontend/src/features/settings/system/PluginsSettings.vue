<script setup lang="ts">
import {
	CheckIcon,
	CodeIcon,
	DownloadIcon,
	ExternalIcon,
	FolderOpenIcon,
	GlobeIcon,
	IssuesIcon,
	PaintbrushIcon,
	PlugIcon,
	RefreshCwIcon,
	SearchIcon,
	SettingsIcon,
	TrashIcon,
	UndoIcon,
	UploadIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Button,
	Collapsible,
	Combobox,
	ConfirmModal,
	defineMessages,
	FilterPills,
	IconButton,
	injectNotificationManager,
	Input,
	SegmentedControl,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onMounted, ref } from 'vue'

import {
	type CatalogPluginEntry,
	clearPluginLogs,
	DEFAULT_PLUGIN_CATALOG_URL,
	exitPluginsSafeMode,
	fetchPluginCatalog,
	getPluginsDir,
	getPluginSettings,
	type InstalledPlugin,
	installedPlugins,
	installPluginRemote,
	installPluginZip,
	listInstalledPlugins,
	type NoctrinthPluginType,
	pluginCatalogUrl,
	pluginLogs,
	pluginRuntimeErrors,
	pluginsSafeMode,
	pluginUpdates,
	quickCssContent,
	quickCssEnabled,
	reloadPlugin,
	removePlugin,
	setPluginEnabled,
	setPluginSetting,
	updateAllPlugins,
} from '@/helpers/noctrinth-plugins'
import { openPath } from '@/helpers/utils.js'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	title: { id: 'app.plugins.title', defaultMessage: 'Plugins' },
	description: {
		id: 'app.plugins.description',
		defaultMessage:
			'Extend Noctrinth with themes, content sources, and sandboxed extensions. Catalog plugins are verified by SHA-256.',
	},
	disclaimerHeader: {
		id: 'app.plugins.disclaimer.header',
		defaultMessage: 'Third-party plugins',
	},
	disclaimerBody: {
		id: 'app.plugins.disclaimer.body',
		defaultMessage:
			'Plugins are created by third-party authors. Noctrinth is not responsible for the safety, reliability, data loss, or any malicious code in installed plugins. Only install plugins from sources you trust.',
	},
	safeModeHeader: {
		id: 'app.plugins.safe-mode.header',
		defaultMessage: 'Safe Mode is active',
	},
	safeModeBody: {
		id: 'app.plugins.safe-mode.body',
		defaultMessage:
			'All plugins and Quick CSS are temporarily paused. Press Ctrl+Shift+Alt+P anytime if a broken theme or plugin locks up the UI.',
	},
	safeModeExit: {
		id: 'app.plugins.safe-mode.exit',
		defaultMessage: 'Exit Safe Mode',
	},
	tabInstalled: { id: 'app.plugins.tab.installed', defaultMessage: 'Installed' },
	tabCatalog: { id: 'app.plugins.tab.catalog', defaultMessage: 'Catalog' },
	installZip: { id: 'app.plugins.action.install-zip', defaultMessage: 'Install from .zip' },
	openFolder: { id: 'app.plugins.action.open-folder', defaultMessage: 'Open folder' },
	browseCatalog: { id: 'app.plugins.action.browse-catalog', defaultMessage: 'Browse catalog' },
	refresh: { id: 'app.plugins.action.refresh', defaultMessage: 'Refresh' },
	reload: { id: 'app.plugins.action.reload', defaultMessage: 'Reload plugin' },
	reloadedToast: { id: 'app.plugins.toast.reloaded', defaultMessage: 'Reloaded "{name}"' },
	updateAll: { id: 'app.plugins.action.update-all', defaultMessage: 'Update all ({count})' },
	updatedAllToast: {
		id: 'app.plugins.toast.updated-all',
		defaultMessage: 'Updated {count} plugin(s)',
	},
	logs: { id: 'app.plugins.action.logs', defaultMessage: 'Plugin logs' },
	logsEmpty: { id: 'app.plugins.logs.empty', defaultMessage: 'No log output yet.' },
	logsClear: { id: 'app.plugins.logs.clear', defaultMessage: 'Clear' },
	quickCssTitle: { id: 'app.plugins.quick-css.title', defaultMessage: 'Quick CSS' },
	quickCssDescription: {
		id: 'app.plugins.quick-css.description',
		defaultMessage: 'Custom CSS applied live on top of the current theme.',
	},
	quickCssClear: { id: 'app.plugins.quick-css.clear', defaultMessage: 'Clear' },
	install: { id: 'app.plugins.action.install', defaultMessage: 'Install' },
	update: { id: 'app.plugins.action.update', defaultMessage: 'Update' },
	installedBadge: { id: 'app.plugins.badge.installed', defaultMessage: 'Installed' },
	localBadge: { id: 'app.plugins.badge.local', defaultMessage: 'Local' },
	typeAll: { id: 'app.plugins.type.all', defaultMessage: 'All' },
	typeTheme: { id: 'app.plugins.type.theme', defaultMessage: 'Theme' },
	typeContentSource: { id: 'app.plugins.type.content-source', defaultMessage: 'Content source' },
	typeExtension: { id: 'app.plugins.type.extension', defaultMessage: 'Extension' },
	emptyInstalledHeading: {
		id: 'app.plugins.empty.installed.heading',
		defaultMessage: 'No plugins installed',
	},
	emptyInstalledDescription: {
		id: 'app.plugins.empty.installed.description',
		defaultMessage: 'Install a verified plugin from the catalog or load a local .zip archive.',
	},
	searchPlaceholder: { id: 'app.plugins.catalog.search', defaultMessage: 'Search plugins...' },
	catalogUrlLabel: { id: 'app.plugins.catalog.url', defaultMessage: 'Catalog index URL' },
	catalogUrlDescription: {
		id: 'app.plugins.catalog.url.description',
		defaultMessage: 'JSON index used to discover and verify community plugins.',
	},
	catalogReset: { id: 'app.plugins.catalog.reset', defaultMessage: 'Reset' },
	catalogLoadError: {
		id: 'app.plugins.catalog.error',
		defaultMessage: 'Could not load the plugin catalog.',
	},
	catalogEmpty: {
		id: 'app.plugins.catalog.empty',
		defaultMessage: 'No plugins match your search.',
	},
	permissionsDomains: {
		id: 'app.plugins.permissions.domains',
		defaultMessage: 'Network: {domains}',
	},
	permissionsEvents: {
		id: 'app.plugins.permissions.events',
		defaultMessage: 'Events: {events}',
	},
	permissionsInstances: {
		id: 'app.plugins.permissions.instances',
		defaultMessage: 'Instances: {perms}',
	},
	permissionsUi: {
		id: 'app.plugins.permissions.ui',
		defaultMessage: 'UI: {perms}',
	},
	pluginSettings: {
		id: 'app.plugins.settings',
		defaultMessage: 'Plugin settings',
	},
	removeConfirmTitle: {
		id: 'app.plugins.remove.title',
		defaultMessage: 'Remove plugin?',
	},
	removeConfirmDesc: {
		id: 'app.plugins.remove.description',
		defaultMessage: 'Remove "{name}" from Noctrinth? Its files will be deleted.',
	},
	removeConfirmButton: {
		id: 'app.plugins.remove.confirm',
		defaultMessage: 'Remove',
	},
	installedToast: {
		id: 'app.plugins.toast.installed',
		defaultMessage: 'Installed "{name}"',
	},
	removedToast: {
		id: 'app.plugins.toast.removed',
		defaultMessage: 'Removed "{name}"',
	},
})

const activeTab = ref<'installed' | 'catalog'>('installed')
const tabItems: ('installed' | 'catalog')[] = ['installed', 'catalog']
function formatTabLabel(tab: 'installed' | 'catalog'): string {
	return tab === 'installed'
		? `${formatMessage(messages.tabInstalled)} (${installedPlugins.value.length})`
		: formatMessage(messages.tabCatalog)
}

const busy = ref(false)
const updatingAll = ref(false)
const reloadingId = ref<string | null>(null)
const installingId = ref<string | null>(null)
const pendingRemove = ref<InstalledPlugin | null>(null)
const removeModal = ref<InstanceType<typeof ConfirmModal> | null>(null)
const openSettingsPluginId = ref<string | null>(null)
const openLogsPluginId = ref<string | null>(null)
const settingsRevision = ref(0)

function readSettingValue(plugin: InstalledPlugin, key: string) {
	void settingsRevision.value
	return getPluginSettings(plugin)[key]
}

function updateSettingValue(pluginId: string, key: string, value: unknown) {
	setPluginSetting(pluginId, key, value)
	settingsRevision.value++
}

function formatLogTime(ts: number): string {
	const d = new Date(ts)
	return d.toTimeString().slice(0, 8)
}

const catalogLoading = ref(false)
const catalogError = ref<string | null>(null)
const catalogItems = ref<CatalogPluginEntry[]>([])
const catalogQuery = ref('')
const catalogTypeFilter = ref<string[]>([])

const installedById = computed(() => {
	const map = new Map<string, InstalledPlugin>()
	for (const p of installedPlugins.value) map.set(p.manifest.id, p)
	return map
})

const updatesById = computed(() => {
	const map = new Map<string, CatalogPluginEntry>()
	for (const entry of pluginUpdates.value) map.set(entry.id, entry)
	return map
})

const typeFilterOptions = computed(() => [
	{ id: 'theme', label: formatMessage(messages.typeTheme) },
	{ id: 'content-source', label: formatMessage(messages.typeContentSource) },
	{ id: 'extension', label: formatMessage(messages.typeExtension) },
])

const filteredCatalog = computed(() => {
	const q = catalogQuery.value.trim().toLowerCase()
	const types = new Set(catalogTypeFilter.value)
	return catalogItems.value.filter((item) => {
		if (types.size > 0 && !types.has(item.type)) return false
		if (!q) return true
		return (
			item.name.toLowerCase().includes(q) ||
			item.id.toLowerCase().includes(q) ||
			(item.description ?? '').toLowerCase().includes(q) ||
			(item.author ?? '').toLowerCase().includes(q) ||
			(item.tags ?? []).some((t) => t.toLowerCase().includes(q))
		)
	})
})

function typeLabel(type: NoctrinthPluginType): string {
	if (type === 'theme') return formatMessage(messages.typeTheme)
	if (type === 'content-source') return formatMessage(messages.typeContentSource)
	return formatMessage(messages.typeExtension)
}

function typeIcon(type: NoctrinthPluginType) {
	if (type === 'theme') return PaintbrushIcon
	if (type === 'content-source') return GlobeIcon
	return PlugIcon
}

function typeBadgeClass(type: NoctrinthPluginType): string {
	if (type === 'theme') return 'bg-highlight-purple text-purple'
	if (type === 'content-source') return 'bg-highlight-blue text-blue'
	return 'bg-highlight-green text-green'
}

async function refreshInstalled() {
	try {
		await listInstalledPlugins()
	} catch (error) {
		handleError(error as Error)
	}
}

async function loadCatalog() {
	catalogLoading.value = true
	catalogError.value = null
	try {
		const data = await fetchPluginCatalog(pluginCatalogUrl.value)
		catalogItems.value = data.plugins
	} catch (error) {
		catalogError.value =
			error instanceof Error ? error.message : formatMessage(messages.catalogLoadError)
	} finally {
		catalogLoading.value = false
	}
}

async function handleInstallZip() {
	const picked = await open({
		multiple: false,
		filters: [{ name: 'Noctrinth Plugin (.zip)', extensions: ['zip'] }],
	})
	if (!picked || Array.isArray(picked)) return
	busy.value = true
	try {
		const installed = await installPluginZip(picked)
		addNotification({
			title: formatMessage(messages.installedToast, { name: installed.manifest.name }),
			type: 'success',
		})
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value = false
	}
}

async function handleInstallFromCatalog(entry: CatalogPluginEntry) {
	if (installingId.value) return
	installingId.value = entry.id
	try {
		const installed = await installPluginRemote(entry.download_url, entry.sha256)
		addNotification({
			title: formatMessage(messages.installedToast, { name: installed.manifest.name }),
			type: 'success',
		})
	} catch (error) {
		handleError(error as Error)
	} finally {
		installingId.value = null
	}
}

async function handleUpdateAll() {
	if (updatingAll.value || pluginUpdates.value.length === 0) return
	updatingAll.value = true
	try {
		const updated = await updateAllPlugins()
		addNotification({
			title: formatMessage(messages.updatedAllToast, { count: updated.length }),
			type: 'success',
		})
	} catch (error) {
		handleError(error as Error)
	} finally {
		updatingAll.value = false
	}
}

async function handleReloadPlugin(plugin: InstalledPlugin) {
	if (reloadingId.value) return
	reloadingId.value = plugin.manifest.id
	try {
		await reloadPlugin(plugin.manifest.id)
		addNotification({
			title: formatMessage(messages.reloadedToast, { name: plugin.manifest.name }),
			type: 'success',
		})
	} catch (error) {
		handleError(error as Error)
	} finally {
		reloadingId.value = null
	}
}

async function handleToggle(plugin: InstalledPlugin, enabled: boolean) {
	try {
		await setPluginEnabled(plugin.manifest.id, enabled)
	} catch (error) {
		handleError(error as Error)
	}
}

function promptRemove(plugin: InstalledPlugin) {
	pendingRemove.value = plugin
	removeModal.value?.show()
}

async function confirmRemove() {
	const target = pendingRemove.value
	if (!target) return
	pendingRemove.value = null
	try {
		await removePlugin(target.manifest.id)
		addNotification({
			title: formatMessage(messages.removedToast, { name: target.manifest.name }),
			type: 'success',
		})
	} catch (error) {
		handleError(error as Error)
	}
}

async function handleOpenPluginsFolder() {
	try {
		const dir = await getPluginsDir()
		await openPath(dir)
	} catch (error) {
		handleError(error as Error)
	}
}

onMounted(() => {
	void refreshInstalled()
	void loadCatalog()
})
</script>

<template>
	<ConfirmModal
		ref="removeModal"
		:title="formatMessage(messages.removeConfirmTitle)"
		:description="
			formatMessage(messages.removeConfirmDesc, {
				name: pendingRemove?.manifest.name ?? '',
			})
		"
		:proceed-label="formatMessage(messages.removeConfirmButton)"
		:proceed-icon="TrashIcon"
		danger
		@proceed="confirmRemove"
	/>

	<div class="flex flex-col gap-6">
		<section class="flex flex-col gap-4">
			<div class="flex flex-wrap items-start justify-between gap-4">
				<div class="min-w-0 flex-1">
					<h2 class="m-0 text-xl font-semibold text-contrast">
						{{ formatMessage(messages.title) }}
					</h2>
					<p class="m-0 mt-1 text-secondary">
						{{ formatMessage(messages.description) }}
					</p>
				</div>
				<SegmentedControl
					v-model="activeTab"
					:items="tabItems"
					:aria-label="formatMessage(messages.title)"
					:format-label="formatTabLabel"
					class="shrink-0"
				/>
			</div>

			<Admonition
				v-if="pluginsSafeMode"
				type="critical"
				:header="formatMessage(messages.safeModeHeader)"
				:body="formatMessage(messages.safeModeBody)"
			>
				<template #actions>
					<Button type="colored" color="brand" size="sm" @click="exitPluginsSafeMode">
						{{ formatMessage(messages.safeModeExit) }}
					</Button>
				</template>
			</Admonition>

			<Admonition
				type="warning"
				:header="formatMessage(messages.disclaimerHeader)"
				:body="formatMessage(messages.disclaimerBody)"
			/>
		</section>

		<template v-if="activeTab === 'installed'">
			<section class="flex flex-col gap-4 border-0 border-t border-solid border-surface-5 pt-6">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<div class="flex flex-wrap items-center gap-2.5">
						<Button type="colored" color="brand" :disabled="busy" @click="handleInstallZip">
							<UploadIcon aria-hidden="true" />
							{{ formatMessage(messages.installZip) }}
						</Button>
						<Button @click="handleOpenPluginsFolder">
							<FolderOpenIcon aria-hidden="true" />
							{{ formatMessage(messages.openFolder) }}
						</Button>
						<Button
							v-if="pluginUpdates.length > 0"
							type="colored"
							color="green"
							:disabled="updatingAll"
							@click="handleUpdateAll"
						>
							<DownloadIcon aria-hidden="true" />
							{{ formatMessage(messages.updateAll, { count: pluginUpdates.length }) }}
						</Button>
					</div>
					<IconButton
						v-tooltip="formatMessage(messages.refresh)"
						:label="formatMessage(messages.refresh)"
						@click="refreshInstalled"
					>
						<RefreshCwIcon />
					</IconButton>
				</div>

				<div v-if="installedPlugins.length > 0" class="flex flex-col gap-3">
					<div
						v-for="plugin in installedPlugins"
						:key="plugin.manifest.id"
						class="flex flex-col gap-3 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-4 transition-colors"
					>
						<div class="flex items-start justify-between gap-4">
							<div class="flex min-w-0 flex-1 items-start gap-3">
								<div
									class="flex size-10 shrink-0 items-center justify-center rounded-xl"
									:class="typeBadgeClass(plugin.manifest.type)"
								>
									<component
										:is="typeIcon(plugin.manifest.type)"
										class="size-5"
										aria-hidden="true"
									/>
								</div>
								<div class="flex min-w-0 flex-1 flex-col gap-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class="font-semibold text-contrast">{{ plugin.manifest.name }}</span>
										<span
											class="rounded-md bg-surface-4 px-1.5 py-0.5 font-mono text-xs text-secondary"
										>
											v{{ plugin.manifest.version }}
										</span>
										<span
											class="rounded-md px-1.5 py-0.5 text-xs font-medium"
											:class="typeBadgeClass(plugin.manifest.type)"
										>
											{{ typeLabel(plugin.manifest.type) }}
										</span>
										<span
											v-if="plugin.source === 'local'"
											class="rounded-md bg-surface-4 px-1.5 py-0.5 text-xs text-secondary"
										>
											{{ formatMessage(messages.localBadge) }}
										</span>
									</div>
									<p v-if="plugin.manifest.description" class="m-0 text-sm text-secondary">
										{{ plugin.manifest.description }}
									</p>
									<div
										v-if="
											plugin.manifest.author ||
											plugin.manifest.permissions?.domains?.length ||
											plugin.manifest.permissions?.events?.length ||
											plugin.manifest.permissions?.instances?.length ||
											plugin.manifest.permissions?.ui?.length
										"
										class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-secondary"
									>
										<span v-if="plugin.manifest.author" class="mr-1 font-medium text-primary">
											{{ plugin.manifest.author }}
										</span>
										<span
											v-if="plugin.manifest.permissions?.domains?.length"
											class="rounded-md bg-surface-2 px-2 py-0.5 font-mono text-[11px]"
										>
											{{
												formatMessage(messages.permissionsDomains, {
													domains: plugin.manifest.permissions.domains.join(', '),
												})
											}}
										</span>
										<span
											v-if="plugin.manifest.permissions?.events?.length"
											class="rounded-md bg-surface-2 px-2 py-0.5 font-mono text-[11px]"
										>
											{{
												formatMessage(messages.permissionsEvents, {
													events: plugin.manifest.permissions.events.join(', '),
												})
											}}
										</span>
										<span
											v-if="plugin.manifest.permissions?.instances?.length"
											class="rounded-md bg-surface-2 px-2 py-0.5 font-mono text-[11px]"
										>
											{{
												formatMessage(messages.permissionsInstances, {
													perms: plugin.manifest.permissions.instances.join(', '),
												})
											}}
										</span>
										<span
											v-if="plugin.manifest.permissions?.ui?.length"
											class="rounded-md bg-surface-2 px-2 py-0.5 font-mono text-[11px]"
										>
											{{
												formatMessage(messages.permissionsUi, {
													perms: plugin.manifest.permissions.ui.join(', '),
												})
											}}
										</span>
									</div>
								</div>
							</div>

							<div class="flex shrink-0 items-center gap-1.5">
								<Button
									v-if="updatesById.has(plugin.manifest.id)"
									type="colored"
									color="green"
									size="sm"
									:disabled="installingId === plugin.manifest.id"
									@click="handleInstallFromCatalog(updatesById.get(plugin.manifest.id)!)"
								>
									<DownloadIcon aria-hidden="true" />
									v{{ updatesById.get(plugin.manifest.id)!.version }}
								</Button>
								<IconButton
									v-tooltip="formatMessage(messages.reload)"
									type="quiet"
									size="sm"
									:label="formatMessage(messages.reload)"
									:disabled="reloadingId === plugin.manifest.id"
									@click="handleReloadPlugin(plugin)"
								>
									<RefreshCwIcon :class="{ 'animate-spin': reloadingId === plugin.manifest.id }" />
								</IconButton>
								<IconButton
									v-if="plugin.manifest.entry || (pluginLogs[plugin.manifest.id]?.length ?? 0) > 0"
									v-tooltip="formatMessage(messages.logs)"
									:type="openLogsPluginId === plugin.manifest.id ? 'base' : 'quiet'"
									size="sm"
									:label="formatMessage(messages.logs)"
									@click="
										openLogsPluginId =
											openLogsPluginId === plugin.manifest.id ? null : plugin.manifest.id
									"
								>
									<CodeIcon />
								</IconButton>
								<IconButton
									v-if="plugin.manifest.settings?.length"
									v-tooltip="formatMessage(messages.pluginSettings)"
									:type="openSettingsPluginId === plugin.manifest.id ? 'base' : 'quiet'"
									size="sm"
									:label="formatMessage(messages.pluginSettings)"
									@click="
										openSettingsPluginId =
											openSettingsPluginId === plugin.manifest.id ? null : plugin.manifest.id
									"
								>
									<SettingsIcon />
								</IconButton>
								<IconButton
									v-if="plugin.manifest.homepage"
									v-tooltip="plugin.manifest.homepage"
									type="quiet"
									size="sm"
									:label="plugin.manifest.homepage"
									@click="openUrl(plugin.manifest.homepage!)"
								>
									<ExternalIcon />
								</IconButton>
								<Toggle
									:model-value="plugin.enabled"
									@update:model-value="(val) => handleToggle(plugin, Boolean(val))"
								/>
								<IconButton
									v-tooltip="formatMessage(messages.removeConfirmButton)"
									type="quiet"
									size="sm"
									:label="formatMessage(messages.removeConfirmButton)"
									class="hover:!text-red"
									@click="promptRemove(plugin)"
								>
									<TrashIcon />
								</IconButton>
							</div>
						</div>

						<Collapsible
							v-if="plugin.manifest.settings?.length"
							:collapsed="openSettingsPluginId !== plugin.manifest.id"
						>
							<div class="flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-3">
								<div
									v-for="field in plugin.manifest.settings"
									:key="field.key"
									class="flex flex-wrap items-center justify-between gap-3"
								>
									<div class="flex min-w-0 flex-1 flex-col gap-0.5">
										<span class="text-sm font-medium text-contrast">{{ field.label }}</span>
										<span v-if="field.description" class="text-xs text-secondary">
											{{ field.description }}
										</span>
									</div>
									<Toggle
										v-if="field.type === 'boolean'"
										:model-value="Boolean(readSettingValue(plugin, field.key))"
										@update:model-value="
											(val) => updateSettingValue(plugin.manifest.id, field.key, Boolean(val))
										"
									/>
									<Combobox
										v-else-if="field.type === 'select' && field.options?.length"
										:model-value="String(readSettingValue(plugin, field.key) ?? '')"
										:options="field.options"
										trigger-size="sm"
										class="!w-48"
										@update:model-value="
											(val) => updateSettingValue(plugin.manifest.id, field.key, String(val))
										"
									/>
									<div v-else-if="field.type === 'color'" class="flex items-center gap-2">
										<input
											type="color"
											class="size-8 cursor-pointer rounded-lg border border-solid border-surface-5 bg-transparent p-0.5"
											:value="String(readSettingValue(plugin, field.key) ?? '#a855f7')"
											@input="
												(e) =>
													updateSettingValue(
														plugin.manifest.id,
														field.key,
														(e.target as HTMLInputElement).value,
													)
											"
										/>
										<Input
											size="small"
											wrapper-class="w-28"
											:model-value="String(readSettingValue(plugin, field.key) ?? '')"
											:placeholder="field.placeholder ?? '#a855f7'"
											@update:model-value="
												(val) => updateSettingValue(plugin.manifest.id, field.key, String(val))
											"
										/>
									</div>
									<Input
										v-else-if="field.type === 'number'"
										type="number"
										size="small"
										wrapper-class="w-32"
										:model-value="String(readSettingValue(plugin, field.key) ?? '')"
										:placeholder="field.placeholder"
										@update:model-value="
											(val) => updateSettingValue(plugin.manifest.id, field.key, Number(val))
										"
									/>
									<Input
										v-else
										size="small"
										wrapper-class="w-64 max-w-full"
										:model-value="String(readSettingValue(plugin, field.key) ?? '')"
										:placeholder="field.placeholder"
										@update:model-value="
											(val) => updateSettingValue(plugin.manifest.id, field.key, String(val))
										"
									/>
								</div>
							</div>
						</Collapsible>

						<Collapsible
							v-if="plugin.manifest.entry || (pluginLogs[plugin.manifest.id]?.length ?? 0) > 0"
							:collapsed="openLogsPluginId !== plugin.manifest.id"
						>
							<div class="flex flex-col gap-2 border-0 border-t border-solid border-surface-5 pt-3">
								<div class="flex items-center justify-between gap-2">
									<span class="text-xs font-semibold uppercase tracking-wide text-secondary">
										{{ formatMessage(messages.logs) }}
									</span>
									<Button
										v-if="pluginLogs[plugin.manifest.id]?.length"
										type="quiet"
										size="sm"
										@click="clearPluginLogs(plugin.manifest.id)"
									>
										{{ formatMessage(messages.logsClear) }}
									</Button>
								</div>
								<div
									class="max-h-44 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2.5 font-mono text-xs"
								>
									<div v-if="!pluginLogs[plugin.manifest.id]?.length" class="text-secondary">
										{{ formatMessage(messages.logsEmpty) }}
									</div>
									<div
										v-for="(entry, idx) in pluginLogs[plugin.manifest.id]"
										:key="idx"
										class="flex items-start gap-2 leading-relaxed"
									>
										<span class="shrink-0 text-secondary">
											{{ formatLogTime(entry.timestamp) }}
										</span>
										<span
											class="shrink-0 font-semibold uppercase"
											:class="
												entry.level === 'error'
													? 'text-red'
													: entry.level === 'warn'
														? 'text-orange'
														: 'text-blue'
											"
										>
											{{ entry.level }}
										</span>
										<span class="min-w-0 break-all text-primary">{{ entry.message }}</span>
									</div>
								</div>
							</div>
						</Collapsible>

						<div
							v-if="pluginRuntimeErrors[plugin.manifest.id]"
							class="flex items-center gap-2 rounded-xl bg-highlight-red px-3 py-2 text-xs text-red"
						>
							<IssuesIcon class="size-4 shrink-0" />
							<span class="min-w-0 break-all">{{ pluginRuntimeErrors[plugin.manifest.id] }}</span>
						</div>
					</div>
				</div>

				<div
					v-else
					class="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-surface-5 bg-surface-2 px-6 py-8 text-center"
				>
					<div
						class="flex size-12 items-center justify-center rounded-2xl bg-surface-3 text-secondary"
					>
						<PlugIcon class="size-6" aria-hidden="true" />
					</div>
					<div class="flex max-w-md flex-col gap-1">
						<h3 class="m-0 text-base font-semibold text-contrast">
							{{ formatMessage(messages.emptyInstalledHeading) }}
						</h3>
						<p class="m-0 text-sm text-secondary">
							{{ formatMessage(messages.emptyInstalledDescription) }}
						</p>
					</div>
					<div class="mt-1">
						<Button type="colored" color="brand" @click="activeTab = 'catalog'">
							<SearchIcon aria-hidden="true" />
							{{ formatMessage(messages.browseCatalog) }}
						</Button>
					</div>
				</div>
			</section>

			<section class="flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-6">
				<div class="flex items-center justify-between gap-4">
					<div class="min-w-0 flex-1">
						<h3 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.quickCssTitle) }}
						</h3>
						<p class="m-0 mt-1 text-sm text-secondary">
							{{ formatMessage(messages.quickCssDescription) }}
						</p>
					</div>
					<Toggle v-model="quickCssEnabled" />
				</div>
				<div v-if="quickCssEnabled" class="flex flex-col gap-2">
					<textarea
						v-model="quickCssContent"
						rows="5"
						spellcheck="false"
						placeholder="/* :root { --color-brand: #a855f7; } */"
						class="w-full resize-y rounded-xl border border-solid border-surface-5 bg-surface-2 p-3 font-mono text-xs text-contrast outline-none focus:border-brand"
					/>
					<div v-if="quickCssContent.trim()" class="flex justify-end">
						<Button type="quiet" size="sm" @click="quickCssContent = ''">
							{{ formatMessage(messages.quickCssClear) }}
						</Button>
					</div>
				</div>
			</section>
		</template>

		<template v-else>
			<section class="flex flex-col gap-4 border-0 border-t border-solid border-surface-5 pt-6">
				<div class="flex flex-wrap items-center gap-2.5">
					<Input
						v-model="catalogQuery"
						:icon="SearchIcon"
						:placeholder="formatMessage(messages.searchPlaceholder)"
						wrapper-class="min-w-[220px] flex-1"
						clearable
					/>
					<Button :disabled="catalogLoading" @click="loadCatalog">
						<RefreshCwIcon :class="{ 'animate-spin': catalogLoading }" aria-hidden="true" />
						{{ formatMessage(messages.refresh) }}
					</Button>
				</div>

				<FilterPills
					:model-value="catalogTypeFilter"
					:options="typeFilterOptions"
					@update:model-value="(v) => (catalogTypeFilter = v)"
				>
					<template #all>{{ formatMessage(messages.typeAll) }}</template>
				</FilterPills>

				<Admonition
					v-if="catalogError"
					type="critical"
					:header="formatMessage(messages.catalogLoadError)"
					:body="catalogError"
				/>

				<div v-else-if="filteredCatalog.length > 0" class="flex flex-col gap-3">
					<div
						v-for="entry in filteredCatalog"
						:key="entry.id"
						class="flex items-start justify-between gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-4"
					>
						<div class="flex min-w-0 flex-1 items-start gap-3">
							<img
								v-if="entry.icon_url"
								:src="entry.icon_url"
								alt=""
								class="size-10 shrink-0 rounded-xl object-cover"
							/>
							<div
								v-else
								class="flex size-10 shrink-0 items-center justify-center rounded-xl"
								:class="typeBadgeClass(entry.type)"
							>
								<component :is="typeIcon(entry.type)" class="size-5" aria-hidden="true" />
							</div>
							<div class="flex min-w-0 flex-1 flex-col gap-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="font-semibold text-contrast">{{ entry.name }}</span>
									<span
										class="rounded-md bg-surface-4 px-1.5 py-0.5 font-mono text-xs text-secondary"
									>
										v{{ entry.version }}
									</span>
									<span
										class="rounded-md px-1.5 py-0.5 text-xs font-medium"
										:class="typeBadgeClass(entry.type)"
									>
										{{ typeLabel(entry.type) }}
									</span>
								</div>
								<p v-if="entry.description" class="m-0 text-sm text-secondary">
									{{ entry.description }}
								</p>
								<div class="flex flex-wrap items-center gap-1.5">
									<span v-if="entry.author" class="mr-1 text-xs font-medium text-primary">
										{{ entry.author }}
									</span>
									<span
										v-for="tag in entry.tags ?? []"
										:key="tag"
										class="rounded-md bg-surface-2 px-1.5 py-0.5 text-[11px] text-secondary"
									>
										#{{ tag }}
									</span>
								</div>
							</div>
						</div>

						<div class="flex shrink-0 items-center gap-2">
							<IconButton
								v-if="entry.homepage || entry.repository"
								v-tooltip="entry.homepage || entry.repository"
								type="quiet"
								size="sm"
								:label="entry.homepage || entry.repository || entry.name"
								@click="openUrl((entry.homepage || entry.repository)!)"
							>
								<ExternalIcon />
							</IconButton>
							<template v-if="installedById.has(entry.id)">
								<Button
									v-if="installedById.get(entry.id)?.manifest.version !== entry.version"
									type="colored"
									color="brand"
									:disabled="installingId === entry.id"
									@click="handleInstallFromCatalog(entry)"
								>
									<DownloadIcon aria-hidden="true" />
									{{ formatMessage(messages.update) }}
								</Button>
								<span
									v-else
									class="flex items-center gap-1.5 rounded-xl bg-surface-4 px-3 py-2 text-sm font-medium text-brand"
								>
									<CheckIcon class="size-4" aria-hidden="true" />
									{{ formatMessage(messages.installedBadge) }}
								</span>
							</template>
							<Button
								v-else
								type="colored"
								color="brand"
								:disabled="installingId === entry.id"
								@click="handleInstallFromCatalog(entry)"
							>
								<DownloadIcon aria-hidden="true" />
								{{ formatMessage(messages.install) }}
							</Button>
						</div>
					</div>
				</div>

				<div
					v-else-if="!catalogLoading"
					class="rounded-2xl border border-dashed border-surface-5 bg-surface-2 px-6 py-8 text-center text-sm text-secondary"
				>
					{{ formatMessage(messages.catalogEmpty) }}
				</div>
			</section>

			<section class="flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-6">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.catalogUrlLabel) }}
					</h3>
					<p class="m-0 mt-1 text-sm text-secondary">
						{{ formatMessage(messages.catalogUrlDescription) }}
					</p>
				</div>
				<div class="flex flex-wrap items-center gap-2.5">
					<Input v-model="pluginCatalogUrl" wrapper-class="min-w-[260px] flex-1" />
					<Button
						@click="
							() => {
								pluginCatalogUrl = DEFAULT_PLUGIN_CATALOG_URL
								loadCatalog()
							}
						"
					>
						<UndoIcon aria-hidden="true" />
						{{ formatMessage(messages.catalogReset) }}
					</Button>
				</div>
			</section>
		</template>
	</div>
</template>
