<script setup lang="ts">
/**
 * A project's description with a translation strip above it: a button that
 * translates it into the reader's language, then a switch between the two.
 *
 * What the strip offers follows Settings → Language: off, it is not there;
 * with the check on, a description already in a language the reader reads
 * shows no button; with auto on, the description arrives translated. The
 * swap is a wave down the paragraphs, each one sharpening out of a blur, and
 * the text shimmers while the service works.
 */
import { LanguagesIcon, SpinnerIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { renderHighlightedString } from '@modrinth/utils'
import { computed, ref, unref, watch } from 'vue'

import {
	detectLanguage,
	languageName,
	needsNoTranslation,
	resolveTarget,
	type TranslatedHtml,
	translateHtml,
	translationSettings,
} from '@/helpers/noctrinth-translate'
import i18n from '@/i18n.config'

const props = defineProps<{ description: string }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	translate: { id: 'noctrinth.translate.button', defaultMessage: 'Translate to {language}' },
	original: { id: 'noctrinth.translate.original', defaultMessage: 'Original' },
	translating: { id: 'noctrinth.translate.translating', defaultMessage: 'Translating...' },
	sourceLanguage: {
		id: 'noctrinth.translate.source-language',
		defaultMessage: 'Written in {language}',
	},
	translatedBy: {
		id: 'noctrinth.translate.translated-by',
		defaultMessage: 'Translated by {service}',
	},
	failed: { id: 'noctrinth.translate.failed', defaultMessage: "Couldn't translate it: {reason}" },
	retry: { id: 'noctrinth.translate.retry', defaultMessage: 'Try again' },
})

const SERVICE_NAMES = { google: 'Google', deepl: 'DeepL' } as const

const locale = computed(() => String(unref(i18n.global.locale)))
const target = computed(() => resolveTarget(locale.value))
const originalHtml = computed(() => renderHighlightedString(props.description ?? ''))

const translation = ref<TranslatedHtml | null>(null)
const showTranslated = ref(false)
const translating = ref(false)
/** The language the check or the translation found the description in. */
const detected = ref<string | null>(null)
/** Nothing to offer: already readable, or the service said so. */
const settled = ref(false)
const error = ref('')

const capitalised = (text: string) => text.charAt(0).toLocaleUpperCase(locale.value) + text.slice(1)
const targetName = computed(() => capitalised(languageName(target.value, locale.value)))
const offered = computed(
	() => translationSettings.enabled && !settled.value && !!props.description?.trim(),
)

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
			detected.value = result.sourceLanguage || detected.value
			showTranslated.value = true
		} else {
			settled.value = true
		}
	} catch (caught) {
		error.value = caught instanceof Error ? caught.message : String(caught)
	} finally {
		translating.value = false
	}
}

let generation = 0
watch(
	[originalHtml, target, () => translationSettings.enabled, () => translationSettings.provider],
	async () => {
		const current = ++generation
		translation.value = null
		showTranslated.value = false
		settled.value = false
		detected.value = null
		error.value = ''
		if (!translationSettings.enabled || !props.description?.trim()) return

		if (translationSettings.precheck) {
			const language = await detectLanguage(originalHtml.value)
			if (current !== generation) return
			detected.value = language
			if (language && needsNoTranslation(language, target.value)) {
				settled.value = true
				return
			}
		}
		if (translationSettings.auto) void translate()
	},
	{ immediate: true },
)
</script>

<template>
	<div class="nt" :class="{ 'is-working': translating }">
		<Transition name="nt-bar">
			<div v-if="offered" class="nt-bar">
				<span class="nt-bar__info">
					<LanguagesIcon class="nt-bar__icon" aria-hidden="true" />
					<Transition name="nt-fade" mode="out-in">
						<span v-if="error" key="error" class="text-red">
							{{ formatMessage(messages.failed, { reason: error }) }}
						</span>
						<span v-else-if="showTranslated && translation" key="by">
							{{
								formatMessage(messages.translatedBy, {
									service: SERVICE_NAMES[translation.provider],
								})
							}}
						</span>
						<span v-else-if="detected" key="lang">
							{{
								formatMessage(messages.sourceLanguage, {
									language: languageName(detected, locale),
								})
							}}
						</span>
					</Transition>
				</span>

				<Transition name="nt-fade" mode="out-in">
					<div
						v-if="translation"
						key="switch"
						class="nt-switch"
						role="radiogroup"
						:class="{ 'is-right': showTranslated }"
					>
						<span class="nt-switch__thumb" aria-hidden="true" />
						<button
							type="button"
							role="radio"
							:aria-checked="!showTranslated"
							:class="{ 'is-active': !showTranslated }"
							@click="showTranslated = false"
						>
							{{ formatMessage(messages.original) }}
						</button>
						<button
							type="button"
							role="radio"
							:aria-checked="showTranslated"
							:class="{ 'is-active': showTranslated }"
							@click="showTranslated = true"
						>
							{{ targetName }}
						</button>
					</div>
					<button
						v-else
						key="pill"
						type="button"
						class="nt-pill"
						:disabled="translating"
						@click="translate"
					>
						<SpinnerIcon v-if="translating" class="animate-spin" aria-hidden="true" />
						<LanguagesIcon v-else aria-hidden="true" />
						<span>
							{{
								translating
									? formatMessage(messages.translating)
									: error
										? formatMessage(messages.retry)
										: formatMessage(messages.translate, { language: languageName(target, locale) })
							}}
						</span>
					</button>
				</Transition>
			</div>
		</Transition>

		<Transition name="nt-swap" mode="out-in">
			<!-- Both sides went through the description sanitiser. -->
			<!-- eslint-disable vue/no-v-html -->
			<div
				:key="showTranslated && translation ? 'translated' : 'original'"
				class="markdown-body nt-body"
				v-html="showTranslated && translation ? translation.html : originalHtml"
			/>
			<!-- eslint-enable vue/no-v-html -->
		</Transition>
	</div>
</template>

<style scoped lang="scss">
.nt-bar {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	justify-content: space-between;
	gap: 0.5rem 1rem;
	margin-bottom: 1rem;
	padding: 0.375rem 0.375rem 0.375rem 0.875rem;
	border: 1px solid var(--surface-5);
	border-radius: 999px;
	background: var(--surface-2);
}

.nt-bar__info {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	min-width: 0;
	font-size: 0.875rem;
	color: var(--color-secondary);
}

.nt-bar__icon {
	width: 1rem;
	height: 1rem;
	flex: none;
	color: var(--color-brand);
}

.nt-pill {
	position: relative;
	display: inline-flex;
	align-items: center;
	gap: 0.5rem;
	overflow: hidden;
	padding: 0.4rem 0.95rem;
	border: 0;
	border-radius: 999px;
	font-weight: 600;
	font-size: 0.875rem;
	color: var(--color-brand);
	background: color-mix(in srgb, var(--color-brand) 14%, transparent);
	cursor: pointer;
	transition:
		background-color 0.2s ease,
		transform 0.15s ease;

	svg {
		width: 1rem;
		height: 1rem;
	}

	&:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-brand) 22%, transparent);
	}

	&:active:not(:disabled) {
		transform: scale(0.97);
	}

	&:disabled {
		cursor: progress;
	}
}

/* A light sweeping across the button while the service works. */
.is-working .nt-pill::after {
	content: '';
	position: absolute;
	inset: 0;
	background: linear-gradient(
		100deg,
		transparent 20%,
		color-mix(in srgb, var(--color-brand) 30%, transparent) 50%,
		transparent 80%
	);
	transform: translateX(-100%);
	animation: nt-sweep 1.2s ease-in-out infinite;
}

.nt-switch {
	position: relative;
	display: inline-grid;
	grid-template-columns: 1fr 1fr;
	padding: 0.2rem;
	border-radius: 999px;
	background: var(--surface-4);

	button {
		position: relative;
		z-index: 1;
		padding: 0.3rem 0.9rem;
		border: 0;
		border-radius: 999px;
		background: none;
		font-weight: 600;
		font-size: 0.875rem;
		color: var(--color-secondary);
		cursor: pointer;
		transition: color 0.25s ease;

		&.is-active {
			color: var(--color-accent-contrast, #fff);
		}
	}
}

.nt-switch__thumb {
	position: absolute;
	top: 0.2rem;
	bottom: 0.2rem;
	left: 0.2rem;
	width: calc(50% - 0.2rem);
	border-radius: 999px;
	background: var(--color-brand);
	box-shadow: 0 2px 8px color-mix(in srgb, var(--color-brand) 35%, transparent);
	transition: transform 0.32s cubic-bezier(0.3, 1.3, 0.5, 1);
}

.nt-switch.is-right .nt-switch__thumb {
	transform: translateX(100%);
}

/* The text itself shimmers and dims a little while it is being translated. */
.nt-body {
	transition: opacity 0.3s ease;
}

.is-working .nt-body {
	opacity: 0.55;
	mask-image: linear-gradient(100deg, #000 30%, rgba(0, 0, 0, 0.35) 50%, #000 70%);
	mask-size: 300% 100%;
	animation: nt-shimmer 1.4s linear infinite;
}

@keyframes nt-sweep {
	to {
		transform: translateX(100%);
	}
}

@keyframes nt-shimmer {
	from {
		mask-position: 100% 0;
	}
	to {
		mask-position: 0 0;
	}
}

/* Out: the whole text blurs away. In: a wave down the paragraphs. */
.nt-swap-leave-active {
	transition:
		opacity 0.14s ease-in,
		filter 0.14s ease-in;
}

.nt-swap-leave-to {
	opacity: 0;
	filter: blur(6px);
}

.nt-swap-enter-active > :deep(*) {
	animation: nt-morph 0.45s cubic-bezier(0.2, 0.7, 0.2, 1) both;
}

@for $i from 1 through 14 {
	.nt-swap-enter-active > :deep(:nth-child(#{$i})) {
		animation-delay: #{($i - 1) * 35}ms;
	}
}

.nt-swap-enter-active > :deep(:nth-child(n + 15)) {
	animation-delay: 490ms;
}

@keyframes nt-morph {
	from {
		opacity: 0;
		filter: blur(8px);
		transform: translateY(6px);
	}
}

.nt-fade-enter-active,
.nt-fade-leave-active {
	transition:
		opacity 0.18s ease,
		transform 0.18s ease;
}

.nt-fade-enter-from,
.nt-fade-leave-to {
	opacity: 0;
	transform: scale(0.96);
}

.nt-bar-enter-active,
.nt-bar-leave-active {
	transition:
		opacity 0.25s ease,
		transform 0.25s ease;
}

.nt-bar-enter-from,
.nt-bar-leave-to {
	opacity: 0;
	transform: translateY(-4px);
}

@media (prefers-reduced-motion: reduce) {
	.nt-swap-enter-active > :deep(*),
	.is-working .nt-pill::after,
	.is-working .nt-body {
		animation: none;
	}

	.nt-switch__thumb {
		transition: none;
	}
}
</style>
