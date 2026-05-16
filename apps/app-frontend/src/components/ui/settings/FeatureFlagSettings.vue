<script setup lang="ts">
import { ButtonStyled, Toggle } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import { DEFAULT_FEATURE_FLAGS, type FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()

const settings = ref(await getSettings())
const options = ref<FeatureFlag[]>(Object.keys(DEFAULT_FEATURE_FLAGS) as FeatureFlag[])

/** Human-readable name and description for each feature flag. */
const FLAG_META: Record<FeatureFlag, { name: string; description: string }> = {
	project_background: {
		name: 'Project background',
		description: "Show a colored gradient based on a project's theme behind its page.",
	},
	page_path: {
		name: 'Show page path',
		description: 'Display the current internal route path in the bottom-left corner — useful for debugging.',
	},
	worlds_tab: {
		name: 'Worlds tab',
		description: 'Add a dedicated Worlds entry to the sidebar navigation.',
	},
	worlds_in_home: {
		name: 'Worlds on home page',
		description: 'Show your recent singleplayer worlds and servers on the home page.',
	},
	server_project_qa: {
		name: 'Server project QA',
		description: 'Enable in-progress quality-assurance features for server projects.',
	},
	server_ram_as_bytes_always_on: {
		name: 'Server RAM in bytes',
		description: 'Always show server memory amounts in raw bytes instead of a friendlier unit.',
	},
	always_show_app_controls: {
		name: 'Always show window controls',
		description: 'Keep the minimize, maximize and close window controls visible at all times.',
	},
	skip_unknown_pack_warning: {
		name: 'Skip unknown pack warning',
		description: 'Do not warn before installing modpacks from unverified sources.',
	},
	i18n_debug: {
		name: 'Translation debug',
		description: 'Overlay translation keys and metadata to help debug localization.',
	},
}

function flagMeta(option: FeatureFlag): { name: string; description: string } {
	return (
		FLAG_META[option] ?? {
			name: option.replaceAll('_', ' '),
			description: '',
		}
	)
}

function setFeatureFlag(key: string, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

watch(
	settings,
	async () => {
		await setSettings(settings.value)
	},
	{ deep: true },
)
</script>
<template>
	<div class="flex flex-col gap-4 min-w-[600px]">
		<div
			v-for="option in options"
			:key="option"
			class="flex items-center justify-between gap-4"
		>
			<div class="min-w-0">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ flagMeta(option).name }}
				</h2>
				<p v-if="flagMeta(option).description" class="m-0 mt-0.5 text-sm text-secondary">
					{{ flagMeta(option).description }}
				</p>
			</div>
			<div class="flex items-center gap-2 shrink-0">
				<ButtonStyled type="transparent">
					<button
						:disabled="themeStore.getFeatureFlag(option) === DEFAULT_FEATURE_FLAGS[option]"
						@click="setFeatureFlag(option, DEFAULT_FEATURE_FLAGS[option])"
					>
						Reset to default
					</button>
				</ButtonStyled>
				<Toggle
					:id="`feature-flag-${option}`"
					:model-value="themeStore.getFeatureFlag(option)"
					@update:model-value="() => setFeatureFlag(option, !themeStore.getFeatureFlag(option))"
				/>
			</div>
		</div>
	</div>
</template>
