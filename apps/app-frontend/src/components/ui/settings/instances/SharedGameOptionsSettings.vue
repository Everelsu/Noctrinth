<script setup lang="ts">
import { UndoIcon } from '@modrinth/assets'
import {
	Button,
	Checkbox,
	DropdownSelect,
	IconButton,
	injectNotificationManager,
	Slider,
	Toggle,
} from '@modrinth/ui'
import { watchDebounced } from '@vueuse/core'
import { computed, ref } from 'vue'

import type { GameOption, GameOptionGroup, SharedGameOptionsProfile } from '@/helpers/game-options'
import {
	emptyProfile,
	fromEntry,
	GAME_OPTION_GROUPS,
	GAME_OPTIONS,
	optionsForGroup,
	toEntry,
} from '@/helpers/game-options'
import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()

const storedProfile: SharedGameOptionsProfile = (await get()).shared_game_options ?? emptyProfile()

const enabled = ref(storedProfile.enabled)

/** Which catalogue options the profile takes over, keyed by `options.txt` key. */
const managed = ref<Record<string, boolean>>({})
/** Current value per option, kept even while an option is unmanaged. */
const values = ref<Record<string, unknown>>({})

for (const option of GAME_OPTIONS) {
	const entry = storedProfile.entries.find((candidate) => candidate.key === option.key)
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

function buildProfile(): SharedGameOptionsProfile {
	return {
		enabled: enabled.value,
		entries: GAME_OPTIONS.filter((option) => managed.value[option.key]).map((option) =>
			toEntry(option, values.value[option.key]),
		),
	}
}

// Debounced so dragging a slider doesn't write the settings row on every frame,
// and re-read first so saving here can't clobber another tab's edits.
watchDebounced(
	[enabled, managed, values],
	async () => {
		try {
			const current = await get()
			await set({ ...current, shared_game_options: buildProfile() })
		} catch (error) {
			handleError(error)
		}
	},
	{ deep: true, debounce: 400, maxWait: 2000 },
)

function choiceValues(option: GameOption): string[] {
	return option.control === 'select' ? option.choices.map((choice) => choice.value) : []
}

function choiceLabel(option: GameOption, value: string): string {
	if (option.control !== 'select') return value
	return option.choices.find((choice) => choice.value === value)?.label ?? value
}

/** Human-readable current value, shown when an option is managed. */
function displayValue(option: GameOption): string {
	const value = values.value[option.key]
	if (option.control === 'toggle') return value ? 'On' : 'Off'
	if (option.control === 'select') return choiceLabel(option, value as string)
	return `${value}${option.unit ?? ''}`
}
</script>

<template>
	<div class="flex flex-col gap-4">
		<div class="rounded-xl bg-bg-raised p-4 flex flex-col gap-3">
			<div class="flex items-start justify-between gap-4">
				<div class="flex flex-col gap-1 min-w-0">
					<h2 class="m-0 text-lg font-semibold text-contrast">Minecraft options</h2>
					<p class="m-0 leading-tight text-secondary">
						Values you switch on here are written into every instance's
						<code>options.txt</code> when it launches. Everything else is left exactly as the game
						saved it.
					</p>
				</div>
				<Toggle id="shared-game-options-enabled" v-model="enabled" class="mt-1 shrink-0" />
			</div>

			<div class="flex items-center justify-between gap-4 flex-wrap">
				<span class="text-sm font-semibold" :class="enabled ? 'text-brand' : 'text-secondary'">
					<template v-if="!enabled">Profile is off — nothing is applied</template>
					<template v-else-if="managedCount === 0">
						Nothing selected yet — tick an option below
					</template>
					<template v-else>
						Applying {{ managedCount }} of {{ GAME_OPTIONS.length }} options
					</template>
				</span>
				<Button v-if="managedCount > 0" type="quiet" size="sm" @click="resetAll">
					<UndoIcon />
					Clear all
				</Button>
			</div>
		</div>

		<details class="rounded-xl bg-bg-raised px-4 py-3">
			<summary class="cursor-pointer font-semibold text-contrast">
				How this behaves across game versions
			</summary>
			<div class="mt-3 flex flex-col gap-2 text-sm leading-normal text-secondary">
				<p class="m-0">
					Minecraft writes every option it knows about into <code>options.txt</code>, so an option
					missing from an instance's file means that version doesn't have it — the launcher leaves
					it alone instead of inventing it. Nothing to configure, it adapts on its own.
				</p>
				<p class="m-0">
					An instance that has never been launched has no file yet, so there is nothing to read. In
					that case the options are written up front, and the game drops anything it doesn't
					recognise the first time it saves.
				</p>
				<p class="m-0">
					A few options kept their name but changed what their value means. Those are marked with a
					version and are only written to versions where they're valid.
				</p>
			</div>
		</details>

		<section
			v-for="group in GAME_OPTION_GROUPS"
			:key="group.id"
			class="rounded-xl bg-bg-raised p-4 flex flex-col gap-1"
		>
			<div class="flex items-baseline justify-between gap-3 mb-2">
				<h3 class="m-0 text-base font-semibold text-contrast">{{ group.label }}</h3>
				<span v-if="managedCountFor(group.id)" class="text-xs font-semibold text-brand">
					{{ managedCountFor(group.id) }} on
				</span>
			</div>

			<div
				v-for="option in optionsForGroup(group.id)"
				:key="option.key"
				class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 rounded-lg px-2 py-2 -mx-2 transition-colors"
				:class="managed[option.key] ? 'bg-surface-2' : 'hover:bg-surface-2/50'"
			>
				<Checkbox
					v-model="managed[option.key]"
					class="min-w-0"
					:description="`Manage ${option.label} across all instances`"
				>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span class="flex items-center gap-2 min-w-0">
							<span
								class="font-semibold truncate"
								:class="managed[option.key] ? 'text-contrast' : 'text-primary'"
								:title="option.key"
							>
								{{ option.label }}
							</span>
							<span
								v-if="option.versionNote"
								class="shrink-0 rounded px-1.5 py-0.5 text-[0.65rem] font-semibold bg-surface-4 text-secondary"
							>
								{{ option.versionNote }}
							</span>
						</span>
						<span v-if="option.description" class="text-xs leading-tight text-secondary">
							{{ option.description }}
						</span>
					</span>
				</Checkbox>

				<div class="flex items-center gap-2 justify-end">
					<span
						v-if="managed[option.key] && option.control !== 'toggle'"
						class="text-xs font-semibold text-secondary tabular-nums w-16 text-right"
					>
						{{ displayValue(option) }}
					</span>

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
						class="w-[11rem]"
						:min="option.min"
						:max="option.max"
						:step="option.step"
						:disabled="!managed[option.key]"
					/>
					<DropdownSelect
						v-else
						v-model="values[option.key] as string"
						:name="`value-${option.key}`"
						class="w-[11rem]"
						:options="choiceValues(option)"
						:display-name="(value: string) => choiceLabel(option, value)"
						:disabled="!managed[option.key]"
					/>

					<IconButton
						v-tooltip="isDefault(option) ? undefined : 'Reset to the game default'"
						type="quiet"
						size="sm"
						label="Reset to default"
						class="shrink-0"
						:disabled="!managed[option.key] || isDefault(option)"
						@click="resetOption(option)"
					>
						<UndoIcon />
					</IconButton>
				</div>
			</div>
		</section>
	</div>
</template>
