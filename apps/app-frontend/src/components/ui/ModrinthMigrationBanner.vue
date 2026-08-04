<script setup lang="ts">
import { DownloadIcon, RightArrowIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, ref } from 'vue'

import {
	get_default_launcher_path,
	get_importable_instances,
	import_instance,
} from '@/helpers/import.js'

const DISMISSED_STORAGE_KEY = 'modrinth-app-migration-dismissed'
const LAUNCHER_TYPE = 'ModrinthApp'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const showCreationModal = inject<(options?: { importMode?: boolean }) => void>('showCreationModal')

const dismissed = ref(localStorage.getItem(DISMISSED_STORAGE_KEY) === 'true')
const basePath = ref<string | null>(null)
const instances = ref<string[]>([])
const importing = ref(false)

const shouldShowBanner = computed(() => !dismissed.value && instances.value.length > 0)

const messages = defineMessages({
	title: {
		id: 'app.migration.modrinth.title',
		defaultMessage: 'Bring your instances over from Modrinth App',
	},
	description: {
		id: 'app.migration.modrinth.description',
		defaultMessage:
			'{count, plural, one {# instance was} other {# instances were}} found in your Modrinth App installation. Importing copies them here — your Modrinth App stays untouched.',
	},
	importAll: {
		id: 'app.migration.modrinth.action.import-all',
		defaultMessage: 'Import all',
	},
	importing: {
		id: 'app.migration.modrinth.action.importing',
		defaultMessage: 'Importing...',
	},
	choose: {
		id: 'app.migration.modrinth.action.choose',
		defaultMessage: 'Choose instances',
	},
	dismiss: {
		id: 'app.migration.modrinth.action.dismiss',
		defaultMessage: 'Dismiss',
	},
})

onMounted(async () => {
	if (dismissed.value) return

	try {
		const path = await get_default_launcher_path(LAUNCHER_TYPE)
		if (!path) return
		const detected = await get_importable_instances(LAUNCHER_TYPE, path)
		if (!detected?.length) return

		basePath.value = path
		instances.value = detected
	} catch {
		// No Modrinth App installation to migrate from
	}
})

function dismissBanner() {
	dismissed.value = true
	localStorage.setItem(DISMISSED_STORAGE_KEY, 'true')
}

async function importAll() {
	if (!basePath.value || importing.value) return

	importing.value = true
	try {
		for (const instance of instances.value) {
			await import_instance(LAUNCHER_TYPE, basePath.value, instance).catch(handleError)
		}
	} finally {
		importing.value = false
	}
	dismissBanner()
}

function chooseInstances() {
	showCreationModal?.({ importMode: true })
	dismissBanner()
}
</script>

<template>
	<section
		v-if="shouldShowBanner"
		class="flex w-full flex-col gap-3 rounded-xl border border-solid border-surface-5 bg-button-bg p-4 text-primary"
	>
		<div class="flex w-full items-center justify-between gap-2">
			<h2 class="m-0 min-w-0 truncate text-base font-semibold text-contrast">
				{{ formatMessage(messages.title) }}
			</h2>
			<button
				type="button"
				class="m-0 flex size-5 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 text-primary transition-colors hover:text-contrast focus-visible:text-contrast"
				:aria-label="formatMessage(messages.dismiss)"
				@click="dismissBanner"
			>
				<XIcon aria-hidden="true" class="size-5" />
			</button>
		</div>
		<p class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.description, { count: instances.length }) }}
		</p>
		<div class="flex flex-wrap gap-2">
			<ButtonStyled color="brand">
				<button :disabled="importing" @click="importAll">
					<DownloadIcon aria-hidden="true" />
					{{ formatMessage(importing ? messages.importing : messages.importAll) }}
				</button>
			</ButtonStyled>
			<ButtonStyled>
				<button :disabled="importing" @click="chooseInstances">
					{{ formatMessage(messages.choose) }}
					<RightArrowIcon aria-hidden="true" />
				</button>
			</ButtonStyled>
		</div>
	</section>
</template>
