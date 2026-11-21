<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { RotateCounterClockwiseIcon, SearchIcon } from '@modrinth/assets'
import { computed, ref, toValue } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import { Button, IconButton } from '#ui/components/base/buttons'
import Combobox, { type ComboboxOption } from '#ui/components/base/Combobox.vue'
import Input from '#ui/components/base/inputs/Input.vue'
import NavTabs from '#ui/components/base/NavTabs.vue'
import Pagination from '#ui/components/base/Pagination.vue'
import ProjectCard from '#ui/components/project/card/ProjectCard.vue'
import ProjectCardList from '#ui/components/project/ProjectCardList.vue'
import SearchFilterControl from '#ui/components/search/SearchFilterControl.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { useStickyObserver } from '#ui/composables/sticky-observer'
import { commonMessages, formatProjectTypeSentence } from '#ui/utils/common-messages'
import type { FilterValue, SortType } from '#ui/utils/search'

import SelectedProjectsFloatingBar from './components/SelectedProjectsFloatingBar.vue'
import {
	SERVER_ENVIRONMENT_FILTER_ID,
	useServerEnvironmentFilter,
} from './composables/use-server-environment-filter'
import BrowseInstallHeader from './header.vue'
import { injectBrowseManager } from './providers/browse-manager'
import type { CardAction } from './types'

const ctx = injectBrowseManager()
const { formatMessage } = useVIntl()
const lockedMessages = computed(() => toValue(ctx.lockedFilterMessages))
const stickyInstallHeaderRef = ref<HTMLElement | null>(null)
const { isStuck: isInstallHeaderStuck } = useStickyObserver(
	stickyInstallHeaderRef,
	'BrowseInstallHeader',
)

const { filterType: serverEnvironmentFilterType, selection: serverEnvironmentSelection } =
	useServerEnvironmentFilter(ctx)

const activeFilterTypes = computed(() => [
	...(ctx.showServerEnvironment?.value ? [serverEnvironmentFilterType.value] : []),
	...ctx.filters.value.filter(
		(f) => f.display !== 'none' && !(ctx.hiddenFilterTypes?.value ?? []).includes(f.id),
	),
])

const activeFilterSelection = computed<FilterValue[]>({
	get: () => [...serverEnvironmentSelection.value, ...ctx.currentFilters.value],
	set: (filters) => {
		const environmentFilters = filters.filter((f) => f.type === SERVER_ENVIRONMENT_FILTER_ID)
		if (environmentFilters.length !== serverEnvironmentSelection.value.length) {
			serverEnvironmentSelection.value = environmentFilters
		}
		ctx.currentFilters.value = filters.filter((f) => f.type !== SERVER_ENVIRONMENT_FILTER_ID)
	},
})

const sortOptions = computed<ComboboxOption<SortType>[]>(() =>
	ctx.effectiveSortTypes.value.map((st) => ({
		value: st,
		label: st.display,
	})),
)

const maxResultsOptions = computed<ComboboxOption<number>[]>(() =>
	(ctx.maxResultsOptions?.value ?? [5, 10, 15, 20, 50, 100]).map((n) => ({
		value: n,
		label: String(n),
	})),
)

const messages = defineMessages({
	searchPlaceholder: {
		id: 'browse.search.placeholder',
		defaultMessage: 'Search {projectType}...',
	},
	viewPrefix: {
		id: 'browse.view-prefix',
		defaultMessage: 'View:',
	},
	filterResults: {
		id: 'browse.filter-results',
		defaultMessage: 'Filter results...',
	},
	offline: {
		id: 'browse.offline',
		defaultMessage: 'You are currently offline. Connect to the internet to browse Modrinth!',
	},
	noResults: {
		id: 'browse.no-results',
		defaultMessage: 'No results found for your query!',
	},
	linkOverridingPreferences: {
		id: 'browse.advanced-filters.link-overriding-preferences',
		defaultMessage: "This link's filters differ from your saved advanced exclusions",
	},
	applySavedPreferences: {
		id: 'browse.advanced-filters.apply-saved-preferences',
		defaultMessage: 'Apply saved preferences',
	},
})

function cardActionType(action: CardAction) {
	if (action.type === 'transparent') return 'quiet'
	if (action.type === 'outlined') return 'outlined'
	return action.color && action.color !== 'standard' ? 'colored' : 'base'
}

function cardActionColor(action: CardAction) {
	const type = cardActionType(action)
	return type === 'colored' || type === 'quiet' ? action.color : undefined
}

function cardActionClass(action: CardAction) {
	const classes: string[] = []
	if (action.type === 'outlined' && action.color && action.color !== 'standard') {
		const outlinedColor = {
			brand: '!text-brand [&_svg]:!text-current !shadow-[inset_0_0_0_1px_var(--color-brand)]',
			red: '!text-red [&_svg]:!text-current !shadow-[inset_0_0_0_1px_var(--color-red)]',
			green: '!text-green [&_svg]:!text-current !shadow-[inset_0_0_0_1px_var(--color-green)]',
		}[action.color]
		if (outlinedColor) classes.push(outlinedColor)
	}
	if (action.hoverLabel) {
		classes.push(
			'group/action !opacity-60 hover:!opacity-100 focus-visible:!opacity-100 !transition-[color,background-color,box-shadow,opacity,transform] !duration-200 !ease-out',
		)
		if (action.hoverColor === 'red') {
			classes.push(
				'hover:!text-red hover:!shadow-[inset_0_0_0_1px_var(--color-red)] hover:!bg-red-highlight focus-visible:!text-red focus-visible:!shadow-[inset_0_0_0_1px_var(--color-red)] focus-visible:!bg-red-highlight',
			)
		}
	}
	return classes.length ? classes.join(' ') : undefined
}

function getLoaderFieldValues(
	result: Labrinth.Search.v3.ResultSearchProject,
	field: string,
): string[] {
	return (result.project_loader_fields?.[field] ?? []).filter(
		(value): value is string => typeof value === 'string',
	)
}

function getProjectCardTags(result: Labrinth.Search.v3.ResultSearchProject, displayOnly: boolean) {
	const tags = new Set(displayOnly ? result.display_categories : result.categories)

	for (const loader of result.loaders) {
		if (loader !== 'mrpack') {
			tags.add(loader)
		}
	}

	if (result.loaders.includes('mrpack')) {
		for (const loader of getLoaderFieldValues(result, 'mrpack_loaders')) {
			tags.add(loader)
		}
	}

	return Array.from(tags)
}
</script>

<template>
	<template v-if="ctx.installContext?.value && ctx.variant !== 'web'">
		<div
			ref="stickyInstallHeaderRef"
			class="browse-install-header sticky top-0 z-20 -mx-6 -mt-6 mb-4 rounded-tl-[--radius-xl] border-0 border-b border-solid bg-surface-1 px-6 py-4 border-surface-5"
			:class="[isInstallHeaderStuck ? 'border-t' : '']"
		>
			<BrowseInstallHeader />
		</div>
	</template>
	<SelectedProjectsFloatingBar v-if="ctx.installContext?.value && ctx.variant !== 'web'" />

	<!-- Noctrinth's own: a place beside the type tabs for the app to choose
	     where results come from. Renders exactly as before when unused. -->
	<div
		:class="$slots['tabs-end'] ? 'flex flex-wrap items-center justify-between gap-2' : 'contents'"
	>
		<NavTabs
			v-if="ctx.showProjectTypeTabs.value"
			:links="ctx.selectableProjectTypes.value"
			:replace="ctx.variant === 'app'"
			:animate-collapse="!$slots['tabs-end']"
		/>
		<slot name="tabs-end" />
	</div>

	<Input
		v-model="ctx.query.value"
		:icon="SearchIcon"
		type="text"
		autocomplete="off"
		:placeholder="
			formatMessage(messages.searchPlaceholder, {
				projectType: formatProjectTypeSentence(formatMessage, ctx.projectType.value, 2),
			})
		"
		clearable
		wrapper-class="w-full"
		size="large"
		@clear="ctx.clearSearch()"
	/>

	<Admonition
		v-if="ctx.linkOverridesAdvancedPrefs.value"
		type="info"
		:header="formatMessage(messages.linkOverridingPreferences)"
		inline-actions
		center-content
	>
		<template #actions>
			<Button type="colored" color="blue" @click="ctx.applySavedAdvancedPrefs()">
				<RotateCounterClockwiseIcon />
				{{ formatMessage(messages.applySavedPreferences) }}
			</Button>
		</template>
	</Admonition>

	<div class="flex flex-wrap items-center gap-2">
		<Combobox
			:model-value="ctx.effectiveCurrentSortType.value"
			:options="sortOptions"
			trigger-type="base"
			:class="
				ctx.variant === 'web'
					? '!w-[16rem] min-w-max max-w-full flex-grow md:flex-grow-0'
					: '!w-[16rem] min-w-max max-w-full'
			"
			@update:model-value="(val: SortType) => (ctx.effectiveCurrentSortType.value = val)"
		>
			<template #prefix>
				<span class="font-semibold text-primary">{{
					formatMessage(commonMessages.sortByLabel)
				}}</span>
			</template>
		</Combobox>

		<Combobox
			:model-value="ctx.maxResults.value"
			:options="maxResultsOptions"
			trigger-type="base"
			:class="
				ctx.variant === 'web'
					? '!w-[9rem] min-w-max max-w-full flex-grow md:flex-grow-0'
					: '!w-[9rem] min-w-max max-w-full'
			"
			:placeholder="formatMessage(commonMessages.viewLabel)"
			@update:model-value="(val: number) => (ctx.maxResults.value = val)"
		>
			<template #prefix>
				<span class="font-semibold text-primary">{{ formatMessage(messages.viewPrefix) }}</span>
			</template>
		</Combobox>

		<div v-if="ctx.filtersMenuOpen && !ctx.filtersMenuOpen.value" class="lg:hidden">
			<Button @click="ctx.filtersMenuOpen.value = true">
				{{ formatMessage(messages.filterResults) }}
			</Button>
		</div>

		<IconButton
			v-if="ctx.cycleDisplayMode"
			label="Change display mode"
			@click="ctx.cycleDisplayMode!()"
		>
			<slot name="display-mode-icon" />
		</IconButton>

		<slot name="toolbar-actions" />

		<Pagination
			:page="ctx.currentPage.value"
			:count="ctx.pageCount.value"
			:class="ctx.variant === 'web' ? 'mx-auto sm:ml-auto sm:mr-0' : 'ml-auto'"
			@switch-page="ctx.setPage"
		/>
	</div>

	<SearchFilterControl
		v-if="ctx.isServerType.value"
		v-model:selected-filters="ctx.serverCurrentFilters.value"
		:filters="ctx.serverFilterTypes.value"
		:project-type="ctx.projectType.value"
		:provided-filters="[]"
		:overridden-provided-filter-types="[]"
	/>
	<SearchFilterControl
		v-else
		v-model:selected-filters="activeFilterSelection"
		:filters="activeFilterTypes"
		:project-type="ctx.projectType.value"
		:provided-filters="ctx.providedFilters?.value ?? []"
		:overridden-provided-filter-types="ctx.overriddenProvidedFilterTypes.value"
		:provided-message="lockedMessages?.providedBy"
	/>

	<div class="search mt-1 [overflow-anchor:none]">
		<section v-if="ctx.loading.value && ctx.loadingComponent" class="offline">
			<component :is="ctx.loadingComponent" />
		</section>
		<ProjectCardList v-else-if="ctx.loading.value" :layout="ctx.effectiveLayout.value">
			<div
				v-for="n in 6"
				:key="`browse-skeleton-${n}`"
				class="flex gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-4 animate-pulse"
				:class="ctx.effectiveLayout.value === 'grid' ? 'flex-col' : 'items-center'"
				aria-hidden="true"
			>
				<div
					class="shrink-0 rounded-2xl bg-surface-3"
					:class="ctx.effectiveLayout.value === 'grid' ? 'h-32 w-full' : 'size-24'"
				/>
				<div class="flex min-w-0 flex-1 flex-col gap-2.5">
					<div class="h-5 w-48 max-w-[70%] rounded-md bg-surface-3" />
					<div class="h-4 w-full max-w-[90%] rounded-md bg-surface-3/80" />
					<div class="mt-1 flex gap-2">
						<div class="h-5 w-16 rounded-full bg-surface-3/70" />
						<div class="h-5 w-20 rounded-full bg-surface-3/70" />
						<div class="h-5 w-14 rounded-full bg-surface-3/70" />
					</div>
				</div>
				<div
					v-if="ctx.effectiveLayout.value !== 'grid'"
					class="flex shrink-0 flex-col items-end gap-2"
				>
					<div class="h-9 w-36 rounded-xl bg-surface-3" />
					<div class="h-4 w-24 rounded-md bg-surface-3/70" />
				</div>
			</div>
		</ProjectCardList>
		<section v-else-if="ctx.offline?.value && ctx.totalHits.value === 0" class="offline">
			{{ formatMessage(messages.offline) }}
		</section>
		<section
			v-else-if="
				ctx.isServerType.value
					? ctx.serverHits.value.length === 0
					: ctx.projectHits.value.length === 0
			"
			class="offline text-center"
		>
			<p>{{ formatMessage(messages.noResults) }}</p>
		</section>

		<ProjectCardList v-else :layout="ctx.effectiveLayout.value">
			<template v-if="ctx.isServerType.value">
				<ProjectCard
					v-for="(result, index) in ctx.serverHits.value"
					:key="`server-card-${result.project_id}`"
					class="browse-card-enter"
					:style="{ '--card-index': Math.min(index, 8) }"
					:title="result.name"
					:icon-url="result.icon_url || undefined"
					:summary="result.summary"
					:tags="result.categories"
					:link="ctx.getServerProjectLink(result)"
					:server-online-players="result.minecraft_java_server?.ping?.data?.players_online ?? 0"
					:server-region="result.minecraft_server?.region"
					:server-recent-plays="result.minecraft_java_server?.verified_plays_2w ?? 0"
					:server-modpack-content="ctx.getServerModpackContent?.(result)"
					:server-ping="ctx.serverPings?.value?.[result.project_id]"
					:server-status-online="!!result.minecraft_java_server?.ping?.data"
					:hide-online-players-label="ctx.variant === 'app'"
					:hide-recent-plays-label="ctx.variant === 'app'"
					:layout="ctx.effectiveLayout.value"
					:max-tags="2"
					is-server-project
					exclude-loaders
					:color="result.color ?? undefined"
					:banner="result.featured_gallery ?? undefined"
					@contextmenu.prevent.stop="(event: MouseEvent) => ctx.onContextMenu?.(event, result)"
					@mouseenter="ctx.onServerProjectHover?.(result)"
					@mouseleave="ctx.onProjectHoverEnd?.()"
				>
					<template v-if="ctx.getCardActions?.(result, ctx.projectType.value)?.length" #actions>
						<div class="flex gap-2">
							<template
								v-for="action in ctx.getCardActions(result, ctx.projectType.value)"
								:key="action.key"
							>
								<IconButton
									v-if="action.circular"
									v-tooltip="action.tooltip"
									:type="cardActionType(action)"
									:color="cardActionColor(action)"
									:class="cardActionClass(action)"
									:label="action.label || action.tooltip || action.key"
									:disabled="action.disabled"
									@click.stop="action.onClick"
								>
									<component :is="action.icon" :class="action.iconClass" />
								</IconButton>
								<Button
									v-else
									v-tooltip="action.tooltip"
									:type="cardActionType(action)"
									:color="cardActionColor(action)"
									:class="cardActionClass(action)"
									:disabled="action.disabled"
									@click.stop="action.onClick"
								>
									<span v-if="action.hoverLabel" class="grid place-items-center overflow-hidden">
										<span
											class="col-start-1 row-start-1 inline-flex items-center gap-2 transition-[opacity,transform] duration-200 ease-out group-hover/action:-translate-y-1.5 group-hover/action:opacity-0 group-focus-visible/action:-translate-y-1.5 group-focus-visible/action:opacity-0"
										>
											<component :is="action.icon" :class="action.iconClass" />
											{{ action.label }}
										</span>
										<span
											class="col-start-1 row-start-1 inline-flex translate-y-1.5 items-center gap-2 opacity-0 transition-[opacity,transform] duration-200 ease-out group-hover/action:translate-y-0 group-hover/action:opacity-100 group-focus-visible/action:translate-y-0 group-focus-visible/action:opacity-100"
										>
											<component :is="action.hoverIcon ?? action.icon" :class="action.iconClass" />
											{{ action.hoverLabel }}
										</span>
									</span>
									<template v-else>
										<component :is="action.icon" :class="action.iconClass" />
										{{ action.label }}
									</template>
								</Button>
							</template>
						</div>
					</template>
				</ProjectCard>
			</template>
			<template v-else>
				<ProjectCard
					v-for="(result, index) in ctx.projectHits.value"
					:key="result.project_id"
					class="browse-card-enter"
					:style="{ '--card-index': Math.min(index, 8) }"
					:link="ctx.getProjectLink(result)"
					:title="result.name"
					:icon-url="result.icon_url ?? undefined"
					:author="{
						name: result.organization == null ? result.author : result.organization,
						link:
							result.organization_id == null
								? `/user/${encodeURIComponent(result.author_id ?? result.author)}`
								: ctx.variant === 'web'
									? `/organization/${result.organization_id}`
									: `https://modrinth.com/organization/${result.organization_id}`,
					}"
					:date-updated="result.date_modified"
					:date-published="result.date_created"
					:displayed-date="
						ctx.effectiveCurrentSortType.value.name === 'newest' ? 'published' : 'updated'
					"
					:downloads="result.downloads"
					:summary="result.summary"
					:tags="getProjectCardTags(result, true)"
					:all-tags="getProjectCardTags(result, false)"
					:deprioritized-tags="ctx.deprioritizedTags.value"
					:exclude-loaders="ctx.excludeLoaders.value"
					:followers="result.follows"
					:banner="result.featured_gallery ?? undefined"
					:color="result.color ?? undefined"
					:environment="
						['mod', 'modpack'].includes(ctx.projectType.value)
							? result.project_loader_fields?.environment?.[0]
							: undefined
					"
					:layout="ctx.effectiveLayout.value"
					@contextmenu.prevent.stop="(event: MouseEvent) => ctx.onContextMenu?.(event, result)"
					@mouseenter="ctx.onProjectHover?.(result)"
					@mouseleave="ctx.onProjectHoverEnd?.()"
				>
					<template v-if="ctx.getCardActions?.(result, ctx.projectType.value)?.length" #actions>
						<div class="flex gap-2">
							<template
								v-for="action in ctx.getCardActions(result, ctx.projectType.value)"
								:key="action.key"
							>
								<IconButton
									v-if="action.circular"
									v-tooltip="action.tooltip"
									:type="cardActionType(action)"
									:color="cardActionColor(action)"
									:class="cardActionClass(action)"
									:label="action.label || action.tooltip || action.key"
									:disabled="action.disabled"
									@click.stop="action.onClick"
								>
									<component :is="action.icon" :class="action.iconClass" />
								</IconButton>
								<Button
									v-else
									v-tooltip="action.tooltip"
									:type="cardActionType(action)"
									:color="cardActionColor(action)"
									:class="cardActionClass(action)"
									:disabled="action.disabled"
									@click.stop="action.onClick"
								>
									<span v-if="action.hoverLabel" class="grid place-items-center overflow-hidden">
										<span
											class="col-start-1 row-start-1 inline-flex items-center gap-2 transition-[opacity,transform] duration-200 ease-out group-hover/action:-translate-y-1.5 group-hover/action:opacity-0 group-focus-visible/action:-translate-y-1.5 group-focus-visible/action:opacity-0"
										>
											<component :is="action.icon" :class="action.iconClass" />
											{{ action.label }}
										</span>
										<span
											class="col-start-1 row-start-1 inline-flex translate-y-1.5 items-center gap-2 opacity-0 transition-[opacity,transform] duration-200 ease-out group-hover/action:translate-y-0 group-hover/action:opacity-100 group-focus-visible/action:translate-y-0 group-focus-visible/action:opacity-100"
										>
											<component :is="action.hoverIcon ?? action.icon" :class="action.iconClass" />
											{{ action.hoverLabel }}
										</span>
									</span>
									<template v-else>
										<component :is="action.icon" :class="action.iconClass" />
										{{ action.label }}
									</template>
								</Button>
							</template>
						</div>
					</template>
				</ProjectCard>
			</template>
		</ProjectCardList>

		<div :class="ctx.variant === 'web' ? 'pagination-after mt-3' : 'flex justify-end mt-3'">
			<Pagination
				:page="ctx.currentPage.value"
				:count="ctx.pageCount.value"
				:class="ctx.variant === 'web' ? 'justify-end' : 'pagination-after'"
				@switch-page="ctx.setPage"
			/>
		</div>
	</div>

	<slot name="after" />
</template>

<style scoped>
.browse-card-enter {
	animation: browse-card-in 210ms cubic-bezier(0.22, 1, 0.36, 1) both;
	animation-delay: calc(var(--card-index, 0) * 22ms);
}

@keyframes browse-card-in {
	from {
		opacity: 0;
		transform: translateY(8px);
	}
	to {
		opacity: 1;
		transform: translateY(0);
	}
}

@media (prefers-reduced-motion: reduce) {
	.browse-card-enter {
		animation: none;
	}
}
</style>
