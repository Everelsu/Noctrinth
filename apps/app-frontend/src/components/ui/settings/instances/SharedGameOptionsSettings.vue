<script setup lang="ts">
import { UndoIcon } from '@modrinth/assets'
import {
	Button,
	Checkbox,
	defineMessages,
	DropdownSelect,
	IconButton,
	Slider,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { watchDebounced } from '@vueuse/core'
import { computed, ref } from 'vue'

import type { GameOption, GameOptionGroup, SharedGameOptionsProfile } from '@/helpers/game-options'
import {
	fromEntry,
	GAME_OPTION_GROUPS,
	GAME_OPTIONS,
	optionsForGroup,
	toEntry,
} from '@/helpers/game-options'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.settings.game-options.title',
		defaultMessage: 'Minecraft options',
	},
	description: {
		id: 'app.settings.game-options.description',
		defaultMessage:
			"Values you switch on here are written into every instance's options.txt when it launches. Everything else is left exactly as the game saved it.",
	},
	profileOff: {
		id: 'app.settings.game-options.profile-off',
		defaultMessage: 'Profile is off — nothing is applied',
	},
	nothingSelected: {
		id: 'app.settings.game-options.nothing-selected',
		defaultMessage: 'Nothing selected yet — tick an option below',
	},
	applyingCount: {
		id: 'app.settings.game-options.applying-count',
		defaultMessage: 'Applying {managed} of {total} options',
	},
	clearAll: {
		id: 'app.settings.game-options.clear-all',
		defaultMessage: 'Clear all',
	},
	groupOnCount: {
		id: 'app.settings.game-options.group-on-count',
		defaultMessage: '{count} on',
	},
	versionsSummary: {
		id: 'app.settings.game-options.versions.summary',
		defaultMessage: 'How this behaves across game versions',
	},
	versionsMissingKeys: {
		id: 'app.settings.game-options.versions.missing-keys',
		defaultMessage:
			"Minecraft writes every option it knows about into options.txt, so an option missing from an instance's file means that version doesn't have it — the launcher leaves it alone instead of inventing it. Nothing to configure, it adapts on its own.",
	},
	versionsNeverLaunched: {
		id: 'app.settings.game-options.versions.never-launched',
		defaultMessage:
			"An instance that has never been launched has no file yet, so there is nothing to read. In that case the options are written up front, and the game drops anything it doesn't recognise the first time it saves.",
	},
	versionsChangedMeaning: {
		id: 'app.settings.game-options.versions.changed-meaning',
		defaultMessage:
			"A few options kept their name but changed what their value means. Those are marked with a version and are only written to versions where they're valid.",
	},
	manageOption: {
		id: 'app.settings.game-options.manage-option',
		defaultMessage: 'Manage {option} across all instances',
	},
	appliesBetween: {
		id: 'app.settings.game-options.applies.between',
		defaultMessage: 'Only applied on Minecraft {min}–{max}.',
	},
	appliesFrom: {
		id: 'app.settings.game-options.applies.from',
		defaultMessage: 'Only applied on Minecraft {min} and newer.',
	},
	appliesUpTo: {
		id: 'app.settings.game-options.applies.up-to',
		defaultMessage: 'Only applied on Minecraft {max} and older.',
	},
	resetTooltip: {
		id: 'app.settings.game-options.reset.tooltip',
		defaultMessage: 'Reset to the game default',
	},
	resetLabel: {
		id: 'app.settings.game-options.reset.label',
		defaultMessage: 'Reset to default',
	},
})

/**
 * The stored profile, owned by the synced settings tab this editor opens from.
 *
 * It used to read and write the settings row itself, which put two writers on
 * one key: the tab that carries the toggle saves the whole settings object, so
 * whichever wrote last won. The tab owns the profile now and this is the editor
 * for it.
 */
const profile = defineModel<SharedGameOptionsProfile>({ required: true })

const enabled = computed(() => profile.value.enabled)

/** Which catalogue options the profile takes over, keyed by `options.txt` key. */
const managed = ref<Record<string, boolean>>({})
/** Current value per option, kept even while an option is unmanaged. */
const values = ref<Record<string, unknown>>({})

for (const option of GAME_OPTIONS) {
	const entry = profile.value.entries.find((candidate) => candidate.key === option.key)
	managed.value[option.key] = entry !== undefined
	values.value[option.key] = entry ? fromEntry(option, entry) : option.default
}

const managedCount = computed(() => Object.values(managed.value).filter(Boolean).length)

function managedCountFor(group: GameOptionGroup): number {
	return optionsForGroup(group).filter((option) => managed.value[option.key]).length
}

function isDefault(option: GameOption): boolean {
	return values.value[option.key] === option.default
}

function resetOption(option: GameOption): void {
	values.value[option.key] = option.default
}

function resetAll(): void {
	for (const option of GAME_OPTIONS) {
		managed.value[option.key] = false
		values.value[option.key] = option.default
	}
}

// Debounced so dragging a slider doesn't rewrite the profile on every frame.
// `profile` itself is deliberately not watched: the toggle that sets `enabled`
// lives outside this editor, and reacting to it here would write straight back.
watchDebounced(
	[managed, values],
	() => {
		profile.value = {
			enabled: profile.value.enabled,
			entries: GAME_OPTIONS.filter((option) => managed.value[option.key]).map((option) =>
				toEntry(option, values.value[option.key]),
			),
		}
	},
	{ deep: true, debounce: 400, maxWait: 2000 },
)

function choiceValues(option: GameOption): string[] {
	return option.control === 'select' ? option.choices.map((choice) => choice.value) : []
}

function choiceLabel(option: GameOption, value: string): string {
	if (option.control !== 'select') return value
	const choice = option.choices.find((candidate) => candidate.value === value)
	return choice ? formatMessage(choice.label) : value
}

/**
 * The line under an option's name.
 *
 * A bare "1.16+" chip beside the label raised more questions than it answered —
 * a version of what? — so the constraint is spelled out in words instead, next
 * to whatever else the option has to say.
 */
function optionNote(option: GameOption): string {
	const parts: string[] = []
	if (option.description) parts.push(formatMessage(option.description))
	if (option.minVersion && option.maxVersion) {
		parts.push(
			formatMessage(messages.appliesBetween, { min: option.minVersion, max: option.maxVersion }),
		)
	} else if (option.minVersion) {
		parts.push(formatMessage(messages.appliesFrom, { min: option.minVersion }))
	} else if (option.maxVersion) {
		parts.push(formatMessage(messages.appliesUpTo, { max: option.maxVersion }))
	}
	return parts.join(' ')
}
</script>

<template>
	<div class="flex flex-col gap-4">
		<div class="rounded-xl bg-bg-raised p-4 flex flex-col gap-3">
			<div class="flex items-center justify-between gap-4 flex-wrap">
				<span class="text-sm font-semibold" :class="enabled ? 'text-brand' : 'text-secondary'">
					<template v-if="!enabled">{{ formatMessage(messages.profileOff) }}</template>
					<template v-else-if="managedCount === 0">
						{{ formatMessage(messages.nothingSelected) }}
					</template>
					<template v-else>
						{{
							formatMessage(messages.applyingCount, {
								managed: managedCount,
								total: GAME_OPTIONS.length,
							})
						}}
					</template>
				</span>
				<Button v-if="managedCount > 0" type="quiet" size="sm" @click="resetAll">
					<UndoIcon />
					{{ formatMessage(messages.clearAll) }}
				</Button>
			</div>
		</div>

		<details class="rounded-xl bg-bg-raised px-4 py-3">
			<summary class="cursor-pointer font-semibold text-contrast">
				{{ formatMessage(messages.versionsSummary) }}
			</summary>
			<div class="mt-3 flex flex-col gap-2 text-sm leading-normal text-secondary">
				<p class="m-0">{{ formatMessage(messages.versionsMissingKeys) }}</p>
				<p class="m-0">{{ formatMessage(messages.versionsNeverLaunched) }}</p>
				<p class="m-0">{{ formatMessage(messages.versionsChangedMeaning) }}</p>
			</div>
		</details>

		<section
			v-for="group in GAME_OPTION_GROUPS"
			:key="group.id"
			class="rounded-xl bg-bg-raised p-4 flex flex-col gap-1"
		>
			<div class="flex items-baseline justify-between gap-3 mb-2">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(group.label) }}
				</h3>
				<span v-if="managedCountFor(group.id)" class="text-xs font-semibold text-brand">
					{{ formatMessage(messages.groupOnCount, { count: managedCountFor(group.id) }) }}
				</span>
			</div>

			<div
				v-for="option in optionsForGroup(group.id)"
				:key="option.key"
				class="grid grid-cols-[minmax(0,1fr)_20rem] items-center gap-4 rounded-lg px-2 py-2.5 -mx-2 transition-colors"
				:class="managed[option.key] ? 'bg-surface-2' : 'hover:bg-surface-2/50'"
			>
				<Checkbox
					v-model="managed[option.key]"
					class="min-w-0"
					:description="
						formatMessage(messages.manageOption, { option: formatMessage(option.label) })
					"
				>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span
							class="font-semibold truncate"
							:class="managed[option.key] ? 'text-contrast' : 'text-primary'"
							:title="option.key"
						>
							{{ formatMessage(option.label) }}
						</span>
						<span v-if="optionNote(option)" class="text-xs leading-tight text-secondary">
							{{ optionNote(option) }}
						</span>
					</span>
				</Checkbox>

				<!-- Every control shares one column so toggles, sliders and
				     dropdowns line up down the page instead of ending wherever
				     their own width happens to put them. -->
				<div class="flex items-center gap-2 justify-end min-w-0">
					<Toggle
						v-if="option.control === 'toggle'"
						:id="`value-${option.key}`"
						v-model="values[option.key] as boolean"
						:disabled="!managed[option.key]"
					/>
					<Slider
						v-else-if="option.control === 'slider'"
						:id="`value-${option.key}`"
						v-model="values[option.key] as number"
						class="w-full"
						:min="option.min"
						:max="option.max"
						:step="option.step"
						:unit="option.unit ? formatMessage(option.unit) : ''"
						:disabled="!managed[option.key]"
					/>
					<DropdownSelect
						v-else
						v-model="values[option.key] as string"
						:name="`value-${option.key}`"
						class="w-full"
						:options="choiceValues(option)"
						:display-name="(value: string) => choiceLabel(option, value)"
						:disabled="!managed[option.key]"
					/>

					<!-- Kept in the layout even when it has nothing to do, so the
					     controls beside it don't shift as values change. -->
					<IconButton
						v-tooltip="formatMessage(messages.resetTooltip)"
						type="quiet"
						size="sm"
						:label="formatMessage(messages.resetLabel)"
						class="shrink-0"
						:class="{ invisible: !managed[option.key] || isDefault(option) }"
						@click="resetOption(option)"
					>
						<UndoIcon />
					</IconButton>
				</div>
			</div>
		</section>
	</div>
</template>
