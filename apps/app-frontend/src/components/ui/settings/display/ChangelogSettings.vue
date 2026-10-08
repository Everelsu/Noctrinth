<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { getChangelog } from '@modrinth/blog'
import { Button, Chips, defineMessages, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import { computed, onMounted, ref } from 'vue'

import { NOCTRINTH_CHANGELOG, type NoctrinthVersionEntry } from '@/helpers/noctrinth-changelog'
import { proxiedFetch } from '@/helpers/proxy-fetch'
import { renderChangelog } from '@/helpers/render-changelog'

const { formatMessage } = useVIntl()

/** Landing page of the fork's author, linked from the changelog header. */
const AUTHOR_URL = 'https://everelsu.github.io/RelsevLink/'

/** Where whoever wants to can support the fork. Offered once, quietly, beside the author. */
const BOOSTY_URL = 'https://boosty.to/relsev'

/** The version this build is, to mark it in the list. */
const installedVersion = ref<string | null>(null)
onMounted(async () => {
	installedVersion.value = await getVersion().catch(() => null)
})

/** Whether an entry is the version running now; a build number after `+` is the same release. */
function isInstalled(version: string | undefined): boolean {
	return !!version && !!installedVersion.value && version.split('+')[0] === installedVersion.value
}

const messages = defineMessages({
	author: { id: 'app.changelog.author', defaultMessage: 'Author' },
	support: { id: 'app.changelog.support', defaultMessage: 'Support on Boosty' },
	installed: { id: 'app.changelog.installed', defaultMessage: 'Installed' },
	sourceNoctrinth: { id: 'app.changelog.source.noctrinth', defaultMessage: 'Noctrinth' },
	sourceModrinth: { id: 'app.changelog.source.modrinth', defaultMessage: 'Modrinth' },
	modrinthNote: {
		id: 'app.changelog.modrinth-note',
		defaultMessage:
			'Showing recent releases. The full Modrinth App changelog is available on their website.',
	},
	openModrinthChangelog: {
		id: 'app.changelog.open-modrinth',
		defaultMessage: 'Open Modrinth changelog',
	},
	noctrinthNote: {
		id: 'app.changelog.noctrinth-note',
		defaultMessage: 'The full Noctrinth changelog is also available online.',
	},
	openNoctrinthChangelog: {
		id: 'app.changelog.open-noctrinth',
		defaultMessage: 'Open Noctrinth changelog',
	},
})

interface ChangelogSection {
	title: string
	/** Rendered markdown, so entries can carry links and screenshots. */
	html: string
}

interface ChangelogEntry {
	version?: string
	date?: string
	sections: ChangelogSection[]
}

type ChangelogSource = 'noctrinth' | 'modrinth'

const source = ref<ChangelogSource>('noctrinth')
const sourceOptions: ChangelogSource[] = ['noctrinth', 'modrinth']

function formatSourceLabel(option: ChangelogSource): string {
	return formatMessage(option === 'noctrinth' ? messages.sourceNoctrinth : messages.sourceModrinth)
}

const NEWLINE = String.fromCharCode(10)

/**
 * Splits a changelog body on its section headings and renders each part.
 *
 * The split exists only so the headings can be styled; everything under one is
 * markdown, so an entry can carry links, emphasis and screenshots.
 */
function parseBody(body: string): ChangelogSection[] {
	const sections: { title: string; lines: string[] }[] = []
	let current: { title: string; lines: string[] } | null = null

	for (const line of body.split(NEWLINE)) {
		const heading = /^\s*#{2,3}\s+(.*)$/.exec(line)
		if (heading) {
			current = { title: heading[1].trim(), lines: [] }
			sections.push(current)
			continue
		}

		if (!current) {
			current = { title: '', lines: [] }
			sections.push(current)
		}
		current.lines.push(line)
	}

	return sections
		.map((section) => ({
			title: section.title,
			html: renderChangelog(section.lines.join(NEWLINE).trim()),
		}))
		.filter((section) => section.title || section.html)
}

/** The same entries as published with the changelog site, from the same file. */
const CHANGELOG_FEED = 'https://everelsu.github.io/Noctrinth/changelog.json'

function isEntry(value: unknown): value is NoctrinthVersionEntry {
	const entry = value as Partial<NoctrinthVersionEntry> | null
	return (
		typeof entry?.version === 'string' &&
		typeof entry.date === 'string' &&
		typeof entry.body === 'string'
	)
}

/**
 * The changelog as it stands on the site, which is newer than this build's
 * whenever a release has come out since, or an entry was corrected. Written in
 * helpers/noctrinth-changelog.ts either way — the site publishes that file — and
 * the copy shipped with the build is what shows offline or until this arrives.
 */
const feed = useQuery({
	queryKey: ['noctrinth-changelog-feed'],
	queryFn: async () => {
		const response = await proxiedFetch(CHANGELOG_FEED, {
			connectTimeout: 8000,
		})
		if (!response.ok) throw new Error(`HTTP ${response.status}`)
		const entries: unknown = await response.json()
		if (!Array.isArray(entries) || !entries.every(isEntry) || entries.length === 0) {
			throw new Error('The changelog feed is not a list of entries')
		}
		return entries as NoctrinthVersionEntry[]
	},
	staleTime: 10 * 60 * 1000,
	retry: 1,
})

const noctrinthChangelog = computed<ChangelogEntry[]>(() =>
	(feed.data.value ?? NOCTRINTH_CHANGELOG).map((entry) => ({
		version: entry.version,
		date: dayjs(entry.date).format('MMM D, YYYY'),
		sections: parseBody(entry.body),
	})),
)

// Modrinth App changelog — pulled from @modrinth/blog, the exact source the
// modrinth.com changelog page renders from. Capped to the most recent releases.
const modrinthChangelog = computed<ChangelogEntry[]>(() =>
	getChangelog()
		.filter((entry) => entry.product === 'app')
		.slice(0, 25)
		.map((entry) => ({
			version: entry.version,
			date: dayjs(entry.date).format('MMM D, YYYY'),
			sections: parseBody(entry.body),
		})),
)

/**
 * Drops a screenshot that did not load.
 *
 * Screenshots are served by the changelog site rather than shipped, so they are
 * missing while offline, and for an entry describing a version whose site build
 * has not run yet. An empty space says less than a broken tile does, and the
 * words are the entry anyway.
 *
 * Listened for in the capture phase because an image's `error` does not bubble.
 */
function hideUnloadableImage(event: Event): void {
	const image = event.target
	if (image instanceof HTMLImageElement) {
		image.remove()
	}
}

const entries = computed<ChangelogEntry[]>(() =>
	source.value === 'noctrinth' ? noctrinthChangelog.value : modrinthChangelog.value,
)
</script>

<template>
	<!-- Screenshots are served by the site, so one listener up here drops any that did not load. -->
	<div class="flex flex-col gap-5" @error.capture="hideUnloadableImage">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<Chips
				v-model="source"
				:items="sourceOptions"
				:format-label="formatSourceLabel"
				:capitalize="false"
				never-empty
			/>
			<div class="flex items-center gap-1.5">
				<Button size="sm" @click="openUrl(AUTHOR_URL)">
					{{ formatMessage(messages.author) }}
					<ExternalIcon aria-hidden="true" />
				</Button>
				<button
					v-tooltip="formatMessage(messages.support)"
					type="button"
					class="nm-press grid size-8 cursor-pointer place-items-center rounded-full border-0 bg-transparent p-0 text-secondary opacity-70 transition-all hover:bg-surface-4 hover:text-contrast hover:opacity-100 focus-visible:opacity-100"
					:aria-label="formatMessage(messages.support)"
					@click="openUrl(BOOSTY_URL)"
				>
					<svg viewBox="0 0 24 24" class="size-4" fill="currentColor" aria-hidden="true">
						<path
							d="M2.661 14.337 6.801 0h6.362L11.88 4.444l-.038.077-3.378 11.733h3.15c-1.321 3.289-2.35 5.867-3.086 7.733-5.816-.063-7.442-4.228-6.02-9.155M8.554 24l7.67-11.035h-3.25l2.83-7.073c4.852.508 7.137 4.33 5.791 8.952C20.16 19.81 14.344 24 8.68 24h-.127z"
						/>
					</svg>
				</button>
			</div>
		</div>

		<section
			v-for="(entry, entryIdx) in entries"
			:key="`${entry.version ?? ''}-${entry.date ?? ''}-${entryIdx}`"
			class="changelog-entry flex flex-col gap-3"
		>
			<div class="flex flex-wrap items-center gap-2">
				<h2 class="m-0 text-xl font-bold text-contrast">
					{{ entry.version ? `v${entry.version}` : entry.date }}
				</h2>
				<span
					v-if="isInstalled(entry.version)"
					class="rounded-full border border-solid border-brand bg-brand-highlight px-2 py-0.5 text-xs font-semibold text-brand"
				>
					{{ formatMessage(messages.installed) }}
				</span>
				<span v-if="entry.version && entry.date" class="text-sm text-secondary">
					{{ entry.date }}
				</span>
			</div>
			<div
				v-for="(section, sectionIdx) in entry.sections"
				:key="sectionIdx"
				class="flex flex-col gap-1.5"
			>
				<h3 v-if="section.title" class="m-0 text-base font-semibold text-brand">
					{{ section.title }}
				</h3>
				<!-- eslint-disable-next-line vue/no-v-html -- ships with the app, sanitised in renderChangelog -->
				<div class="changelog-body text-sm text-primary" v-html="section.html" />
			</div>
		</section>

		<!-- Link to the full changelog for the selected source -->
		<div v-if="source === 'modrinth'" class="flex flex-col gap-2">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.modrinthNote) }}
			</p>
			<Button @click="openUrl('https://modrinth.com/news/changelog?filter=app')">
				<ExternalIcon />
				{{ formatMessage(messages.openModrinthChangelog) }}
			</Button>
		</div>
		<div v-else class="flex flex-col gap-2">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.noctrinthNote) }}
			</p>
			<Button @click="openUrl('https://everelsu.github.io/Noctrinth/')">
				<ExternalIcon />
				{{ formatMessage(messages.openNoctrinthChangelog) }}
			</Button>
		</div>
	</div>
</template>
<style scoped lang="scss">
.changelog-entry + .changelog-entry {
	padding-top: 1.25rem;
	border-top: 1px solid var(--surface-5);
}

/*
 * Rendered markdown has no classes to hang utilities on, so the body is styled
 * here: links in the brand colour so they read as links, code as code, and
 * list items with room between them.
 */
:deep(.changelog-body a) {
	color: var(--color-brand);
	font-weight: 500;
	text-decoration: underline;
	text-decoration-color: color-mix(in srgb, var(--color-brand) 40%, transparent);
	text-underline-offset: 2px;
	transition: text-decoration-color 120ms ease-out;
}

:deep(.changelog-body a:hover) {
	text-decoration-color: currentColor;
}

:deep(.changelog-body code) {
	padding: 0.05rem 0.35rem;
	border-radius: 0.375rem;
	background: var(--surface-4);
	color: var(--color-contrast);
	font-size: 0.85em;
}

:deep(.changelog-body ul) {
	margin: 0;
	padding-left: 1.25rem;
}

:deep(.changelog-body li + li) {
	margin-top: 0.35rem;
}

:deep(.changelog-body li::marker) {
	color: var(--color-secondary);
}

:deep(.changelog-body p) {
	margin: 0;
}

/*
 * Screenshots arrive at whatever size they were taken, which is wider than this
 * panel — a window capture on a high-density display more than twice as wide.
 * Sized to the column and left to keep its own proportions.
 */
:deep(.changelog-body img) {
	display: block;
	max-width: 100%;
	height: auto;
	margin-top: 0.5rem;
	border: 1px solid var(--color-button-bg);
	border-radius: var(--radius-md);
}
</style>
