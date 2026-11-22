<script setup lang="ts">
/** Translating project descriptions: which service, into what, and when. */
import { CheckIcon, LanguagesIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	Collapsible,
	Combobox,
	type ComboboxOption,
	defineMessages,
	Input,
	MultiSelect,
	type MultiSelectOption,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, unref } from 'vue'

import {
	COMMON_LANGUAGES,
	languageName,
	languageOfLocale,
	resolveTarget,
	testTranslation,
	type TranslationProviderId,
	translationSettings,
} from '@/helpers/noctrinth-translate'
import i18n from '@/i18n.config'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'noctrinth.settings.translation.title', defaultMessage: 'Translate descriptions' },
	description: {
		id: 'noctrinth.settings.translation.description',
		defaultMessage:
			"Offers to translate a project's description into your language, with a switch back to the original.",
	},
	service: { id: 'noctrinth.settings.translation.service', defaultMessage: 'Service' },
	google: {
		id: 'noctrinth.settings.translation.google',
		defaultMessage: 'Google Translate: free, no key needed',
	},
	deepl: {
		id: 'noctrinth.settings.translation.deepl',
		defaultMessage: 'DeepL: more natural, needs your own free API key',
	},
	deeplKey: { id: 'noctrinth.settings.translation.deepl-key', defaultMessage: 'DeepL API key' },
	deeplKeyHint: {
		id: 'noctrinth.settings.translation.deepl-key-hint',
		defaultMessage: 'A free key translates 500,000 characters a month. Get one at',
	},
	target: { id: 'noctrinth.settings.translation.target', defaultMessage: 'Translate into' },
	targetApp: {
		id: 'noctrinth.settings.translation.target-app',
		defaultMessage: "The launcher's language ({language})",
	},
	skip: { id: 'noctrinth.settings.translation.skip', defaultMessage: 'Languages you read anyway' },
	skipHint: {
		id: 'noctrinth.settings.translation.skip-hint',
		defaultMessage: 'Descriptions in these are left as they are.',
	},
	auto: {
		id: 'noctrinth.settings.translation.auto',
		defaultMessage: 'Translate as soon as a description opens',
	},
	precheck: {
		id: 'noctrinth.settings.translation.precheck',
		defaultMessage:
			'Check the language first, and hide the button when there is nothing to translate',
	},
	test: { id: 'noctrinth.settings.translation.test', defaultMessage: 'Test the service' },
	testOk: {
		id: 'noctrinth.settings.translation.test-ok',
		defaultMessage: 'Works, {milliseconds} ms: "{text}"',
	},
})

const locale = computed(() => String(unref(i18n.global.locale)))
const name = (code: string) => {
	const text = languageName(code, locale.value)
	return text.charAt(0).toLocaleUpperCase(locale.value) + text.slice(1)
}

const providerOptions = computed<ComboboxOption<TranslationProviderId>[]>(() => [
	{ value: 'google', label: formatMessage(messages.google) },
	{ value: 'deepl', label: formatMessage(messages.deepl) },
])

const targetOptions = computed<ComboboxOption<string>[]>(() => [
	{
		value: 'app',
		label: formatMessage(messages.targetApp, { language: name(languageOfLocale(locale.value)) }),
	},
	...COMMON_LANGUAGES.map((code) => ({ value: code, label: name(code) })),
])

const skipOptions = computed<MultiSelectOption<string>[]>(() =>
	COMMON_LANGUAGES.map((code) => ({ value: code, label: name(code) })),
)

const testing = ref(false)
const testResult = ref<{ ok: boolean; text: string } | null>(null)

async function runTest() {
	testing.value = true
	testResult.value = null
	try {
		const { text, milliseconds } = await testTranslation(resolveTarget(locale.value))
		testResult.value = { ok: true, text: formatMessage(messages.testOk, { milliseconds, text }) }
	} catch (error) {
		testResult.value = { ok: false, text: error instanceof Error ? error.message : String(error) }
	} finally {
		testing.value = false
	}
}
</script>

<template>
	<section class="flex flex-col gap-4">
		<div class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<h2
					id="noctrinth-translation-label"
					class="m-0 flex items-center gap-2 text-xl font-semibold text-contrast"
				>
					<LanguagesIcon class="size-5 text-secondary" aria-hidden="true" />
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 mt-1 text-secondary">
					{{ formatMessage(messages.description) }}
				</p>
			</div>
			<Toggle
				id="noctrinth-translation-enabled"
				v-model="translationSettings.enabled"
				aria-labelledby="noctrinth-translation-label"
			/>
		</div>

		<Collapsible :collapsed="!translationSettings.enabled" overflow-visible>
			<div
				class="flex flex-col gap-5 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-4"
			>
				<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div class="flex flex-col gap-2">
						<h3
							id="noctrinth-translation-service"
							class="m-0 text-base font-semibold text-contrast"
						>
							{{ formatMessage(messages.service) }}
						</h3>
						<Combobox
							v-model="translationSettings.provider"
							:options="providerOptions"
							aria-labelledby="noctrinth-translation-service"
							class="w-full"
							@update:model-value="testResult = null"
						/>
					</div>

					<div class="flex flex-col gap-2">
						<h3 id="noctrinth-translation-target" class="m-0 text-base font-semibold text-contrast">
							{{ formatMessage(messages.target) }}
						</h3>
						<Combobox
							v-model="translationSettings.target"
							:options="targetOptions"
							aria-labelledby="noctrinth-translation-target"
							searchable
							class="w-full"
						/>
					</div>
				</div>

				<div v-if="translationSettings.provider === 'deepl'" class="flex flex-col gap-1.5">
					<label for="noctrinth-deepl-key" class="text-base font-semibold text-contrast">
						{{ formatMessage(messages.deeplKey) }}
					</label>
					<Input
						id="noctrinth-deepl-key"
						v-model="translationSettings.deeplKey"
						type="password"
						autocomplete="off"
						placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx"
						wrapper-class="w-full"
					/>
					<span class="text-sm text-secondary">
						{{ formatMessage(messages.deeplKeyHint) }}
						<a
							href="https://www.deepl.com/pro-api"
							target="_blank"
							rel="noopener noreferrer"
							class="font-semibold text-brand hover:underline"
							>deepl.com</a
						>
					</span>
				</div>

				<div class="flex flex-col gap-2">
					<div>
						<h3 class="m-0 text-base font-semibold text-contrast">
							{{ formatMessage(messages.skip) }}
						</h3>
						<p class="m-0 mt-0.5 text-sm text-secondary">{{ formatMessage(messages.skipHint) }}</p>
					</div>
					<MultiSelect
						v-model="translationSettings.skip"
						:options="skipOptions"
						:placeholder="formatMessage(messages.skip)"
						searchable
						clearable
						class="w-full"
					/>
				</div>

				<div class="flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-4">
					<div class="flex items-center justify-between gap-4">
						<span id="noctrinth-translation-auto" class="font-medium text-primary">
							{{ formatMessage(messages.auto) }}
						</span>
						<Toggle
							v-model="translationSettings.auto"
							aria-labelledby="noctrinth-translation-auto"
						/>
					</div>
					<div class="flex items-center justify-between gap-4">
						<span id="noctrinth-translation-precheck" class="font-medium text-primary">
							{{ formatMessage(messages.precheck) }}
						</span>
						<Toggle
							v-model="translationSettings.precheck"
							aria-labelledby="noctrinth-translation-precheck"
						/>
					</div>
				</div>

				<div class="flex flex-wrap items-center gap-3">
					<Button :disabled="testing" @click="runTest">
						<SpinnerIcon v-if="testing" class="animate-spin" aria-hidden="true" />
						<LanguagesIcon v-else aria-hidden="true" />
						{{ formatMessage(messages.test) }}
					</Button>
					<span
						v-if="testResult"
						class="flex items-center gap-1.5 text-sm"
						:class="testResult.ok ? 'text-green' : 'text-red'"
					>
						<CheckIcon v-if="testResult.ok" class="size-4 shrink-0" aria-hidden="true" />
						<XIcon v-else class="size-4 shrink-0" aria-hidden="true" />
						{{ testResult.text }}
					</span>
				</div>
			</div>
		</Collapsible>
	</section>
</template>
