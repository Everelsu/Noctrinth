<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { getChangelog } from '@modrinth/blog'
import { Button, Chips, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import { computed, onMounted, ref } from 'vue'

import { getNoctrinthChangelog, refreshNoctrinthChangelog } from '@/helpers/noctrinth-changelog'
import { renderChangelog } from '@/helpers/render-changelog'

const { formatMessage } = useVIntl()

/** Landing page of the fork's author, linked from the changelog header. */
const AUTHOR_URL = 'https://everelsu.github.io/RelsevLink/'

const messages = defineMessages({
	author: { id: 'app.changelog.author', defaultMessage: 'Author' },
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

// Noctrinth changelog — the copy bundled with this build, replaced when the
// changelog site answers with a newer one.
const noctrinthEntries = ref(getNoctrinthChangelog())

// The bundled entries are on screen already, so this only fills in what has
// been written since this build; offline it quietly does nothing.
onMounted(() => {
	void refreshNoctrinthChangelog().then((changed) => {
		if (changed) {
			noctrinthEntries.value = getNoctrinthChangelog()
		}
	})
})

const noctrinthChangelog = computed<ChangelogEntry[]>(() =>
	noctrinthEntries.value.map((entry) => ({
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
			<Button size="sm" @click="openUrl(AUTHOR_URL)">
				{{ formatMessage(messages.author) }}
				<ExternalIcon aria-hidden="true" />
			</Button>
		</div>

		<section
			v-for="(entry, entryIdx) in entries"
			:key="`${entry.version ?? ''}-${entry.date ?? ''}-${entryIdx}`"
			class="flex flex-col gap-3"
		>
			<div class="flex items-baseline gap-2">
				<h2 class="m-0 text-xl font-bold text-contrast">
					{{ entry.version ? `v${entry.version}` : entry.date }}
				</h2>
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
