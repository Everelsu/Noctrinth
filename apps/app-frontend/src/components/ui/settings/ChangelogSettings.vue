<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { getChangelog } from '@modrinth/blog'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import { computed, ref } from 'vue'

import { getNoctrinthChangelog } from '@/helpers/noctrinth-changelog'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	sourceNoctrinth: { id: 'app.changelog.source.noctrinth', defaultMessage: 'Noctrinth' },
	sourceModrinth: { id: 'app.changelog.source.modrinth', defaultMessage: 'Modrinth' },
	modrinthNote: {
		id: 'app.changelog.modrinth-note',
		defaultMessage:
			'Showing recent releases. The full Modrinth App changelog is available on their website.',
	},
	openNoctrinthChangelog: {
		id: 'app.changelog.open-noctrinth',
		defaultMessage: 'Open Noctrinth changelog',
	},
})

interface ChangelogSection {
	title: string
	items: string[]
}

interface ChangelogEntry {
	version?: string
	date?: string
	sections: ChangelogSection[]
}

type ChangelogSource = 'noctrinth' | 'modrinth'

const source = ref<ChangelogSource>('noctrinth')

/** Parses a changelog markdown body into titled sections. */
function parseBody(body: string): ChangelogSection[] {
	const sections: ChangelogSection[] = []
	let current: ChangelogSection | null = null

	for (const rawLine of body.split('\n')) {
		const line = rawLine.trim()
		if (!line) continue

		if (line.startsWith('### ')) {
			current = { title: line.slice(4).trim(), items: [] }
			sections.push(current)
			continue
		}
		if (line.startsWith('## ')) {
			current = { title: line.slice(3).trim(), items: [] }
			sections.push(current)
			continue
		}

		if (!current) {
			current = { title: '', items: [] }
			sections.push(current)
		}

		current.items.push(line.replace(/^[-*]\s+/, ''))
	}

	return sections
}

// Noctrinth changelog — pulled from src/helpers/noctrinth-changelog.ts.
const noctrinthChangelog = computed<ChangelogEntry[]>(() =>
	getNoctrinthChangelog().map((entry) => ({
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

const entries = computed<ChangelogEntry[]>(() =>
	source.value === 'noctrinth' ? noctrinthChangelog.value : modrinthChangelog.value,
)
</script>

<template>
	<div class="flex flex-col gap-5 min-w-[600px]">
		<!-- Source toggle -->
		<div class="flex w-fit gap-1 rounded-xl bg-button-bg p-1">
			<button
				class="cursor-pointer rounded-lg border-0 px-3 py-1.5 text-sm font-semibold transition-colors"
				:class="
					source === 'noctrinth'
						? 'bg-brand text-brand-inverted'
						: 'bg-transparent text-secondary hover:text-contrast'
				"
				@click="source = 'noctrinth'"
			>
				{{ formatMessage(messages.sourceNoctrinth) }}
			</button>
			<button
				class="cursor-pointer rounded-lg border-0 px-3 py-1.5 text-sm font-semibold transition-colors"
				:class="
					source === 'modrinth'
						? 'bg-brand text-brand-inverted'
						: 'bg-transparent text-secondary hover:text-contrast'
				"
				@click="source = 'modrinth'"
			>
				{{ formatMessage(messages.sourceModrinth) }}
			</button>
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
				<ul class="m-0 pl-5 flex flex-col gap-1">
					<li v-for="(item, idx) in section.items" :key="idx" class="text-sm text-primary">
						{{ item }}
					</li>
				</ul>
			</div>
		</section>

		<!-- Link to the authoritative Modrinth changelog -->
		<div v-if="source === 'modrinth'" class="flex flex-col gap-2">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.modrinthNote) }}
			</p>
			<ButtonStyled>
				<button @click="openUrl('https://everelsu.github.io/Noctrinth/')">
					<ExternalIcon />
					{{ formatMessage(messages.openNoctrinthChangelog) }}
				</button>
			</ButtonStyled>
		</div>
	</div>
</template>
