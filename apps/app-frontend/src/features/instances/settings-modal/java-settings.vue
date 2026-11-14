<script setup lang="ts">
import {
	CheckCircleIcon,
	DownloadIcon,
	IssuesIcon,
	RocketIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, Input, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'

import JavaSelector from '@/features/java/java-selector/index.vue'
import MemorySlider from '@/features/java/memory-slider.vue'
import { javaVersionsQueryOptions, jreQueryOptions } from '@/features/java/queries'
import type { ModernJavaStatus } from '@/helpers/instance'
import {
	edit,
	get_modern_java_status,
	install_modern_java,
	remove_modern_java,
} from '@/helpers/instance'
import { appSettingsQueryOptions, parseEnvVars, serializeEnvVars } from '@/helpers/settings.ts'
import { instanceRecommendedJavaQueryOptions } from '@/pages/instance/query-options'
import { autoInstallJava } from '@/platform/app-lib/jre/commands'

import DefaultSettingsIntro from './default-settings-intro.vue'
import { injectInstanceSettings } from './instance-settings-context'
import ResetToDefaultButton from './reset-to-default-button.vue'
import { useInstanceSettingsDraft } from './use-instance-settings-draft'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const { instance } = injectInstanceSettings()

const draft = useInstanceSettingsDraft(
	() => ({
		javaPath: instance.value.java_path ?? null,
		javaArgs: instance.value.extra_launch_args?.join(' ') ?? null,
		envVars: instance.value.custom_env_vars
			? serializeEnvVars(instance.value.custom_env_vars)
			: null,
		memory: instance.value.memory?.maximum ?? null,
	}),
	(value) =>
		edit(instance.value.id, {
			java_path: value.javaPath ? value.javaPath.replace('java.exe', 'javaw.exe') : null,
			extra_launch_args:
				value.javaArgs !== null ? value.javaArgs.trim().split(/\s+/).filter(Boolean) : null,
			custom_env_vars: value.envVars !== null ? parseEnvVars(value.envVars) : null,
			memory: value.memory !== null ? { maximum: value.memory } : null,
		}),
)

const settingsQuery = useQuery(appSettingsQueryOptions())
const javaVersionsQuery = useQuery(javaVersionsQueryOptions())
const recommendedJavaQuery = useQuery(
	computed(() => instanceRecommendedJavaQueryOptions(instance.value)),
)

await Promise.all([
	settingsQuery.suspense(),
	javaVersionsQuery.suspense(),
	recommendedJavaQuery.suspense(),
]).catch(handleError)

const recommendedJavaVersion = computed(() => recommendedJavaQuery.data.value)
const defaultJavaPath = computed(() =>
	recommendedJavaVersion.value
		? (javaVersionsQuery.data.value?.[recommendedJavaVersion.value]?.path ?? '')
		: '',
)
const defaultJavaArgs = computed(() => settingsQuery.data.value?.extra_launch_args.join(' ') ?? '')
const defaultEnvVars = computed(() => serializeEnvVars(settingsQuery.data.value?.custom_env_vars))
const defaultMemory = computed(() => settingsQuery.data.value?.memory.maximum ?? 0)

const modernJava = ref<ModernJavaStatus | null>(
	(await get_modern_java_status(instance.value.id).catch(handleError)) ?? null,
)
const modernJavaBusy = ref(false)
const activeJavaQuery = useQuery(
	computed(() => ({
		...jreQueryOptions(draft.value.javaPath ?? defaultJavaPath.value),
		enabled: !!(draft.value.javaPath ?? defaultJavaPath.value),
	})),
)
const activeJavaMajor = computed(
	() => activeJavaQuery.data.value?.parsed_version ?? recommendedJavaVersion.value,
)
async function applyModernJava(action: typeof install_modern_java) {
	modernJavaBusy.value = true
	try {
		modernJava.value = await action(instance.value.id)
		await recommendedJavaQuery.refetch()
		await javaVersionsQuery.refetch()
	} catch (error) {
		handleError(error)
	} finally {
		modernJavaBusy.value = false
	}
}
async function selectJavaMajor(major: number) {
	if (major === activeJavaMajor.value || modernJavaBusy.value) return
	modernJavaBusy.value = true
	try {
		draft.value.javaPath = await autoInstallJava(major)
	} catch (error) {
		handleError(error)
	} finally {
		modernJavaBusy.value = false
	}
}

const messages = defineMessages({
	defaultSettingsDescription: {
		id: 'instance.settings.tabs.java.default-settings-description',
		defaultMessage:
			'Your instance uses the default Java and memory settings unless you override them here.',
	},
	modernJava: {
		id: 'instance.settings.tabs.java.modern-java',
		defaultMessage: 'Modern Java',
	},
	modernJavaBeta: {
		id: 'instance.settings.tabs.java.modern-java-beta',
		defaultMessage: 'Beta',
	},
	modernJavaBetaNote: {
		id: 'instance.settings.tabs.java.modern-java-beta-note',
		defaultMessage:
			'This is new and not widely tested yet. Back up the instance before enabling it, and expect mods that assume Java 8 to misbehave.',
	},
	modernJavaDescription: {
		id: 'instance.settings.tabs.java.modern-java-description',
		defaultMessage:
			'Minecraft {gameVersion} normally only runs on Java 8. {provider} replaces LWJGL 2 and reworks Forge so the instance can run on a current Java release instead.',
	},
	modernJavaInstalled: {
		id: 'instance.settings.tabs.java.modern-java-installed',
		defaultMessage: 'Installed — {provider} {version}, running on Java {javaMajor}',
	},
	modernJavaInstall: {
		id: 'instance.settings.tabs.java.modern-java-install',
		defaultMessage: 'Enable',
	},
	modernJavaInstalling: {
		id: 'instance.settings.tabs.java.modern-java-installing',
		defaultMessage: 'Enabling...',
	},
	modernJavaRemove: {
		id: 'instance.settings.tabs.java.modern-java-remove',
		defaultMessage: 'Disable',
	},
	modernJavaVersion: {
		id: 'instance.settings.tabs.java.modern-java-version',
		defaultMessage: 'Java version',
	},
	modernJavaVersionHint: {
		id: 'instance.settings.tabs.java.modern-java-version-hint',
		defaultMessage:
			'Picking a version downloads it. Select Save to use it for this instance. The lowest version is the best tested; newer ones are supported but less proven with mods for this Minecraft version.',
	},
	modernJavaModsNote: {
		id: 'instance.settings.tabs.java.modern-java-mods-note',
		defaultMessage:
			'Disabling removes the launcher patches. Any mods it installed stay in the instance — remove them from the Mods tab if you no longer want them.',
	},
	javaInstallation: {
		id: 'instance.settings.tabs.java.java-installation',
		defaultMessage: 'Java installation',
	},
	recommendedJavaVersion: {
		id: 'instance.settings.tabs.java.recommended-java-version',
		defaultMessage: 'Java {version} is recommended for the Minecraft version of this instance.',
	},
	javaPathPlaceholder: {
		id: 'instance.settings.tabs.java.java-path-placeholder',
		defaultMessage: '/path/to/java',
	},
	javaMemory: {
		id: 'instance.settings.tabs.java.java-memory',
		defaultMessage: 'Memory allocation',
	},
	javaArguments: {
		id: 'instance.settings.tabs.java.java-arguments',
		defaultMessage: 'Java arguments',
	},
	enterJavaArguments: {
		id: 'instance.settings.tabs.java.enter-java-arguments',
		defaultMessage: 'Enter Java arguments...',
	},
	javaEnvironmentVariables: {
		id: 'instance.settings.tabs.java.environment-variables',
		defaultMessage: 'Environment variables',
	},
	enterEnvironmentVariables: {
		id: 'instance.settings.tabs.java.enter-environment-variables',
		defaultMessage: 'Enter environmental variables...',
	},
	hooks: {
		id: 'instance.settings.tabs.java.hooks',
		defaultMessage: 'Hooks',
	},
})
</script>

<template>
	<div class="flex flex-col gap-6">
		<DefaultSettingsIntro
			tab="java"
			:description="formatMessage(messages.defaultSettingsDescription)"
		/>

		<section v-if="modernJava?.supported" class="flex flex-col">
			<h2 class="m-0 mb-2 text-lg font-semibold text-contrast flex items-center gap-2">
				{{ formatMessage(messages.modernJava) }}
				<span
					class="inline-flex items-center rounded-full border border-solid border-surface-5 bg-surface-4 px-2 py-0.5 text-xs font-semibold text-orange"
				>
					{{ formatMessage(messages.modernJavaBeta) }}
				</span>
			</h2>
			<div class="flex gap-3 items-start p-4 bg-bg rounded-2xl">
				<div
					class="w-10 h-10 flex items-center justify-center rounded-full bg-button-bg border-solid border-[1px] border-button-border p-2 mt-1 shrink-0 [&_svg]:h-full [&_svg]:w-full"
				>
					<RocketIcon />
				</div>
				<div class="flex flex-col gap-2 flex-1 min-w-0">
					<p class="m-0 text-secondary">
						{{
							formatMessage(messages.modernJavaDescription, {
								gameVersion: instance.game_version,
								provider: modernJava.provider_name,
							})
						}}
					</p>
					<p
						v-if="modernJava.installed"
						class="m-0 font-semibold text-contrast flex items-center gap-2"
					>
						<CheckCircleIcon class="h-4 w-4 text-green" />
						{{
							formatMessage(messages.modernJavaInstalled, {
								provider: modernJava.provider_name,
								version: modernJava.installed_version ?? '?',
								javaMajor: activeJavaMajor ?? modernJava.java_major ?? '?',
							})
						}}
					</p>
					<p class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.modernJavaBetaNote) }}
					</p>
					<p
						v-if="modernJava.loader_warning"
						class="m-0 text-sm text-orange flex items-start gap-2"
					>
						<IssuesIcon class="h-4 w-4 shrink-0 mt-[0.15rem]" />
						{{ modernJava.loader_warning }}
					</p>
					<template v-if="modernJava.installed && modernJava.java_majors.length > 1">
						<span class="font-semibold mt-1">
							{{ formatMessage(messages.modernJavaVersion) }}
						</span>
						<div class="flex gap-2 flex-wrap">
							<Button
								v-for="major in modernJava.java_majors"
								:key="major"
								:color="major === activeJavaMajor ? 'primary' : undefined"
								:disabled="modernJavaBusy"
								@click="selectJavaMajor(major)"
							>
								Java {{ major }}
							</Button>
						</div>
						<p class="m-0 text-sm text-secondary">
							{{ formatMessage(messages.modernJavaVersionHint) }}
						</p>
					</template>
					<p v-if="modernJava.installed" class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.modernJavaModsNote) }}
					</p>
					<div class="flex gap-2 mt-1">
						<Button
							v-if="!modernJava.installed"
							color="primary"
							:disabled="modernJavaBusy"
							@click="applyModernJava(install_modern_java)"
						>
							<SpinnerIcon v-if="modernJavaBusy" class="animate-spin" />
							<DownloadIcon v-else />
							{{
								formatMessage(
									modernJavaBusy ? messages.modernJavaInstalling : messages.modernJavaInstall,
								)
							}}
						</Button>
						<Button v-else :disabled="modernJavaBusy" @click="applyModernJava(remove_modern_java)">
							<SpinnerIcon v-if="modernJavaBusy" class="animate-spin" />
							<TrashIcon v-else />
							{{ formatMessage(messages.modernJavaRemove) }}
						</Button>
					</div>
				</div>
			</div>
		</section>

		<section class="flex flex-col">
			<div class="flex items-center justify-between gap-4">
				<h2 class="m-0 min-w-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.javaMemory) }}
				</h2>
				<ResetToDefaultButton
					:disabled="draft.memory === null"
					:setting="formatMessage(messages.javaMemory)"
					@reset="draft.memory = null"
				/>
			</div>
			<div class="pt-3">
				<MemorySlider
					id="max-memory"
					:model-value="draft.memory ?? defaultMemory"
					:aria-label="formatMessage(messages.javaMemory)"
					@update:model-value="(value) => (draft.memory = value)"
				/>
			</div>
		</section>

		<section class="flex flex-col">
			<div class="flex items-center justify-between gap-4">
				<div class="flex min-w-0 flex-col gap-1">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.javaInstallation) }}
					</h2>
					<p v-if="recommendedJavaVersion" class="m-0 text-secondary">
						{{
							formatMessage(messages.recommendedJavaVersion, { version: recommendedJavaVersion })
						}}
					</p>
				</div>
				<ResetToDefaultButton
					:disabled="draft.javaPath === null"
					:setting="formatMessage(messages.javaInstallation)"
					@reset="draft.javaPath = null"
				/>
			</div>
			<div class="pt-3">
				<JavaSelector
					id="java-installation"
					:version="recommendedJavaVersion"
					:model-value="{ path: draft.javaPath ?? defaultJavaPath }"
					:placeholder="formatMessage(messages.javaPathPlaceholder)"
					@update:model-value="(val) => (draft.javaPath = val?.path ?? '')"
				/>
			</div>
		</section>

		<section class="flex flex-col">
			<div class="flex items-center justify-between gap-4">
				<h2 class="m-0 min-w-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.javaArguments) }}
				</h2>
				<ResetToDefaultButton
					:disabled="draft.javaArgs === null"
					:setting="formatMessage(messages.javaArguments)"
					@reset="draft.javaArgs = null"
				/>
			</div>
			<div class="pt-3">
				<Input
					id="java-args"
					:model-value="draft.javaArgs ?? defaultJavaArgs"
					autocomplete="off"
					:placeholder="formatMessage(messages.enterJavaArguments)"
					wrapper-class="w-full"
					@update:model-value="(value) => (draft.javaArgs = String(value ?? ''))"
				/>
			</div>
		</section>

		<section class="flex flex-col">
			<div class="flex items-center justify-between gap-4">
				<h2 class="m-0 min-w-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.javaEnvironmentVariables) }}
				</h2>
				<ResetToDefaultButton
					:disabled="draft.envVars === null"
					:setting="formatMessage(messages.javaEnvironmentVariables)"
					@reset="draft.envVars = null"
				/>
			</div>
			<div class="pt-3">
				<Input
					id="env-vars"
					:model-value="draft.envVars ?? defaultEnvVars"
					autocomplete="off"
					:placeholder="formatMessage(messages.enterEnvironmentVariables)"
					wrapper-class="w-full"
					@update:model-value="(value) => (draft.envVars = String(value ?? ''))"
				/>
			</div>
		</section>
	</div>
</template>
