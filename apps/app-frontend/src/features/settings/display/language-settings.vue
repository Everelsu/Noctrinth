<template>
	<div class="flex flex-col">
		<SharedLanguageSettings ref="languageSettings" product="app" :persist-locale="persistLocale" />
		<NoctrinthTranslationSettings
			class="mt-8 border-0 border-t border-solid border-surface-5 pt-6"
		/>
	</div>
</template>

<script setup lang="ts">
import { LanguageSettings as SharedLanguageSettings } from '@modrinth/ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
import { get, set } from '@/helpers/settings.ts'

import NoctrinthTranslationSettings from './NoctrinthTranslationSettings.vue'

const settingsModal = injectAppSettingsModal()
const languageSettings = ref<InstanceType<typeof SharedLanguageSettings> | null>(null)

onMounted(() => {
	settingsModal.registerUnsavedChangesController({
		hasChanges: () => languageSettings.value?.hasChanges ?? false,
		getOriginal: () => languageSettings.value?.originalState ?? {},
		getModified: () => languageSettings.value?.modifiedState ?? {},
		isSaving: () => languageSettings.value?.saving ?? false,
		reset: () => languageSettings.value?.reset(),
		save: () => languageSettings.value?.save(),
	})
})

onBeforeUnmount(() => {
	settingsModal.registerUnsavedChangesController(null)
})

async function persistLocale(locale: string): Promise<void> {
	const settings = await get()
	if (settings.locale === locale) return
	await set({ ...settings, locale })
}
</script>
