<script setup lang="ts">
/**
 * A project's description with a button that translates it into the
 * launcher's language, and back. With "translate automatically" on, it
 * arrives translated. A description already in that language shows no button
 * once Google has said so.
 */
import { LanguagesIcon, SpinnerIcon, UndoIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { renderHighlightedString } from '@modrinth/utils'
import { computed, ref, unref, watch } from 'vue'

import {
	languageName,
	readAutoTranslate,
	type TranslatedHtml,
	translateHtml,
	translationTarget,
} from '@/helpers/noctrinth-translate'
import i18n from '@/i18n.config'

const props = defineProps<{ description: string }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	translate: {
		id: 'noctrinth.translate.button',
		defaultMessage: 'Translate to {language}',
	},
	original: { id: 'noctrinth.translate.original', defaultMessage: 'Show original' },
	translating: { id: 'noctrinth.translate.translating', defaultMessage: 'Translating...' },
	translatedFrom: {
		id: 'noctrinth.translate.translated-from',
		defaultMessage: 'Translated from {language} by Google',
	},
	failed: {
		id: 'noctrinth.translate.failed',
		defaultMessage: "Couldn't translate it: {reason}",
	},
})

const locale = computed(() => String(unref(i18n.global.locale)))
const target = computed(() => translationTarget(locale.value))
const originalHtml = computed(() => renderHighlightedString(props.description ?? ''))

const translation = ref<TranslatedHtml | null>(null)
const showTranslated = ref(false)
const translating = ref(false)
/** Google said it is in the reader's language already: nothing to offer. */
const alreadyInTarget = ref(false)
const error = ref('')

async function translate() {
	if (translation.value) {
		showTranslated.value = true
		return
	}
	translating.value = true
	error.value = ''
	try {
		const result = await translateHtml(originalHtml.value, target.value)
		if (result) {
			translation.value = result
			showTranslated.value = true
		} else {
			alreadyInTarget.value = true
		}
	} catch (caught) {
		error.value = caught instanceof Error ? caught.message : String(caught)
	} finally {
		translating.value = false
	}
}

watch(
	[originalHtml, target],
	() => {
		translation.value = null
		showTranslated.value = false
		alreadyInTarget.value = false
		error.value = ''
		if (readAutoTranslate() && props.description?.trim()) void translate()
	},
	{ immediate: true },
)
</script>

<template>
	<div>
		<div
			v-if="!alreadyInTarget && description?.trim()"
			class="mb-4 flex flex-wrap items-center gap-x-3 gap-y-1"
		>
			<Button v-if="showTranslated" size="sm" @click="showTranslated = false">
				<UndoIcon aria-hidden="true" />
				{{ formatMessage(messages.original) }}
			</Button>
			<Button v-else size="sm" :disabled="translating" @click="translate">
				<SpinnerIcon v-if="translating" class="animate-spin" aria-hidden="true" />
				<LanguagesIcon v-else aria-hidden="true" />
				{{
					translating
						? formatMessage(messages.translating)
						: formatMessage(messages.translate, { language: languageName(target, locale) })
				}}
			</Button>
			<span v-if="showTranslated && translation" class="text-sm text-secondary">
				{{
					formatMessage(messages.translatedFrom, {
						language: languageName(translation.sourceLanguage, locale),
					})
				}}
			</span>
			<span v-if="error" class="text-sm text-red">
				{{ formatMessage(messages.failed, { reason: error }) }}
			</span>
		</div>
		<!-- Both sides went through the description sanitiser. -->
		<!-- eslint-disable vue/no-v-html -->
		<div
			class="markdown-body"
			v-html="showTranslated && translation ? translation.html : originalHtml"
		/>
		<!-- eslint-enable vue/no-v-html -->
	</div>
</template>
