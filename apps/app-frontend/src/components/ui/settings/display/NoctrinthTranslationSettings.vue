<script setup lang="ts">
/** Translating project descriptions: which service, into what, and when. */
import { CheckIcon, LanguagesIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	Combobox,
	type ComboboxOption,
	defineMessages,
	Input,
	RadioButtons,
	SettingsToggleCard,
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

const targetOptions = computed<ComboboxOption<string>[]>(() => [
	{
		value: 'app',
		label: formatMessage(messages.targetApp, { language: name(languageOfLocale(locale.value)) }),
	},
	...COMMON_LANGUAGES.map((code) => ({ value: code, label: name(code) })),
])

function toggleSkip(code: string) {
	const index = translationSettings.skip.indexOf(code)
	if (index >= 0) translationSettings.skip.splice(index, 1)
	else translationSettings.skip.push(code)
}

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

const providers: TranslationProviderId[] = ['google', 'deepl']
</script>

<template>
	<SettingsToggleCard
		v-model="translationSettings.enabled"
		:icon="LanguagesIcon"
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
	>
		<template #expanded>
			<section class="flex flex-col gap-2">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.service) }}
				</h3>
				<RadioButtons
					v-model="translationSettings.provider"
					:items="providers"
					@update:model-value="testResult = null"
				>
					<template #default="{ item }">
						{{ formatMessage(messages[item as TranslationProviderId]) }}
					</template>
				</RadioButtons>
				<div v-if="translationSettings.provider === 'deepl'" class="flex flex-col gap-1 pl-1">
					<label for="noctrinth-deepl-key" class="font-semibold text-contrast">
						{{ formatMessage(messages.deeplKey) }}
					</label>
					<Input
						id="noctrinth-deepl-key"
						v-model="translationSettings.deeplKey"
						type="password"
						autocomplete="off"
						placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx"
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
			</section>

			<section class="flex flex-col gap-2">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.target) }}
				</h3>
				<Combobox
					v-model="translationSettings.target"
					:options="targetOptions"
					searchable
					class="max-w-sm"
				/>
			</section>

			<section class="flex flex-col gap-2">
				<div>
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.skip) }}
					</h3>
					<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.skipHint) }}</p>
				</div>
				<div class="flex flex-wrap gap-1.5">
					<button
						v-for="code in COMMON_LANGUAGES"
						:key="code"
						type="button"
						class="skip-chip"
						:class="{ 'is-on': translationSettings.skip.includes(code) }"
						:aria-pressed="translationSettings.skip.includes(code)"
						@click="toggleSkip(code)"
					>
						<CheckIcon v-if="translationSettings.skip.includes(code)" aria-hidden="true" />
						{{ name(code) }}
					</button>
				</div>
			</section>

			<section class="flex flex-col gap-3">
				<label class="flex items-center justify-between gap-4">
					<span class="text-primary">{{ formatMessage(messages.auto) }}</span>
					<Toggle v-model="translationSettings.auto" />
				</label>
				<label class="flex items-center justify-between gap-4">
					<span class="text-primary">{{ formatMessage(messages.precheck) }}</span>
					<Toggle v-model="translationSettings.precheck" />
				</label>
			</section>

			<section class="flex flex-wrap items-center gap-3">
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
			</section>
		</template>
	</SettingsToggleCard>
</template>

<style scoped lang="scss">
.skip-chip {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
	padding: 0.3rem 0.75rem;
	border: 1px solid var(--surface-5);
	border-radius: 999px;
	background: var(--surface-3);
	font-size: 0.875rem;
	font-weight: 500;
	color: var(--color-secondary);
	cursor: pointer;
	transition:
		background-color 0.2s ease,
		color 0.2s ease,
		border-color 0.2s ease;

	svg {
		width: 0.9rem;
		height: 0.9rem;
	}

	&:hover {
		color: var(--color-contrast);
	}

	&.is-on {
		border-color: color-mix(in srgb, var(--color-brand) 45%, transparent);
		background: color-mix(in srgb, var(--color-brand) 15%, transparent);
		color: var(--color-brand);
	}
}
</style>
