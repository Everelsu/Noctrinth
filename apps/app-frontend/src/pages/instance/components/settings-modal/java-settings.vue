<script setup lang="ts">
import {
	CheckCircleIcon,
	CoffeeIcon,
	DownloadIcon,
	FolderSearchIcon,
	IssuesIcon,
	RefreshCwIcon,
	RocketIcon,
	SearchIcon,
	SpinnerIcon,
	TrashIcon,
	XCircleIcon,
} from '@modrinth/assets'
import {
	Button,
	Checkbox,
	defineMessages,
	injectNotificationManager,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import JavaDetectionModal from '@/components/ui/JavaDetectionModal.vue'
import useJavaTest from '@/composables/useJavaTest'
import useMemorySlider from '@/composables/useMemorySlider'
import type { JavaVersion, ModernJavaStatus } from '@/helpers/instance'
import {
	edit,
	get_modern_java_status,
	get_optimal_jre_key,
	install_modern_java,
	remove_modern_java,
} from '@/helpers/instance'
import { auto_install_java, get_jre } from '@/helpers/jre.js'
import { get } from '@/helpers/settings.ts'

import type { AppSettings } from '../../../../helpers/types'
import { injectInstanceSettings } from './instance-settings-context'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const { instance } = injectInstanceSettings()

const globalSettings = (await get().catch(handleError)) as unknown as AppSettings

const optimalJava = ref<JavaVersion | null>(
	(await get_optimal_jre_key(instance.value.id).catch(handleError)) ?? null,
)

const overrideJavaInstall = ref(!!instance.value.java_path)
const javaPath = ref(instance.value.java_path ?? optimalJava.value?.path ?? '')

const activePath = computed(() =>
	overrideJavaInstall.value ? javaPath.value : (optimalJava.value?.path ?? ''),
)

watch(overrideJavaInstall, (enabled) => {
	if (enabled && !javaPath.value) {
		javaPath.value = optimalJava.value?.path ?? ''
	}
})

const modernJava = ref<ModernJavaStatus | null>(
	(await get_modern_java_status(instance.value.id).catch(handleError)) ?? null,
)
const modernJavaBusy = ref(false)

// Installing or removing the patches changes which Java the instance asks for,
// so the recommended installation shown above has to be re-resolved.
async function applyModernJava(action: typeof install_modern_java) {
	modernJavaBusy.value = true
	try {
		const status = await action(instance.value.id)
		modernJava.value = status
		optimalJava.value = (await get_optimal_jre_key(instance.value.id)) ?? null
		if (!overrideJavaInstall.value) {
			javaPath.value = optimalJava.value?.path ?? ''
		}
	} catch (error) {
		handleError(error)
	} finally {
		modernJavaBusy.value = false
	}
}

// The Java the instance will really launch with: the override when there is
// one, otherwise whatever the patches resolved to.
const activeJavaMajor = computed(() => optimalJava.value?.parsed_version ?? null)

// Picking a runtime pins it as the instance's Java installation, which is also
// what the section below then shows.
async function selectJavaMajor(major: number) {
	if (major === activeJavaMajor.value || modernJavaBusy.value) {
		return
	}

	modernJavaBusy.value = true
	try {
		const path = await auto_install_java(major)
		overrideJavaInstall.value = true
		javaPath.value = path
		optimalJava.value = (await get_jre(path)) ?? optimalJava.value
	} catch (error) {
		handleError(error)
	} finally {
		modernJavaBusy.value = false
	}
}

const { testingJava, javaTestResult, testJavaInstallationDebounced, testJavaInstallation } =
	useJavaTest()

const hoveringTest = ref(false)
let hasInitialized = false

watch(
	[activePath, optimalJava],
	([newPath]) => {
		if (newPath && optimalJava.value?.parsed_version) {
			if (!hasInitialized) {
				testJavaInstallation(newPath, optimalJava.value.parsed_version, false)
				hasInitialized = true
			} else {
				testJavaInstallationDebounced(newPath, optimalJava.value.parsed_version)
			}
		}
	},
	{ immediate: true },
)

const javaDetectionModal = ref<{ show: (version: number, current: object) => void } | null>(null)

async function handleBrowseJava() {
	const result = await open({ multiple: false })
	if (result) {
		javaPath.value = result
	}
}

function handleDetectJava() {
	javaDetectionModal.value?.show(optimalJava.value?.parsed_version, { path: javaPath.value })
}

const overrideJavaArgs = ref((instance.value.extra_launch_args?.length ?? 0) > 0)
const javaArgs = ref(
	(instance.value.extra_launch_args ?? globalSettings.extra_launch_args).join(' '),
)

const overrideEnvVars = ref((instance.value.custom_env_vars?.length ?? 0) > 0)
const envVars = ref(
	(instance.value.custom_env_vars ?? globalSettings.custom_env_vars)
		.map((x) => x.join('='))
		.join(' '),
)

const overrideMemorySettings = ref(!!instance.value.memory)
const memory = ref(instance.value.memory ?? globalSettings.memory)
const { maxMemory, snapPoints } = (await useMemorySlider().catch(handleError)) as unknown as {
	maxMemory: number
	snapPoints: number[]
}

const editInstanceObject = computed(() => {
	return {
		java_path:
			overrideJavaInstall.value && javaPath.value
				? javaPath.value.replace('java.exe', 'javaw.exe')
				: null,
		extra_launch_args: overrideJavaArgs.value
			? javaArgs.value.trim().split(/\s+/).filter(Boolean)
			: null,
		custom_env_vars: overrideEnvVars.value
			? envVars.value
					.trim()
					.split(/\s+/)
					.filter(Boolean)
					.map((x) => x.split('=').filter(Boolean))
			: null,
		memory: overrideMemorySettings.value ? memory.value : null,
	}
})

watch(
	[
		overrideJavaInstall,
		javaPath,
		overrideJavaArgs,
		javaArgs,
		overrideEnvVars,
		envVars,
		overrideMemorySettings,
		memory,
	],
	async () => {
		await edit(instance.value.id, editInstanceObject.value)
	},
	{ deep: true },
)

const messages = defineMessages({
	modernJava: {
		id: 'instance.settings.tabs.java.modern-java',
		defaultMessage: 'Modern Java',
	},
	modernJavaDescription: {
		id: 'instance.settings.tabs.java.modern-java-description',
		defaultMessage:
			'Minecraft {gameVersion} normally only runs on Java 8. lwjgl3ify replaces LWJGL 2 and patches Forge so the instance can run on a current Java release instead.',
	},
	modernJavaInstalled: {
		id: 'instance.settings.tabs.java.modern-java-installed',
		defaultMessage: 'Installed — lwjgl3ify {version}, running on Java {javaMajor}',
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
			'Picking a version downloads it and pins it as this instance’s Java installation. The lowest one is the best tested; newer ones are supported but less proven with 1.7.10 mods.',
	},
	modernJavaModsNote: {
		id: 'instance.settings.tabs.java.modern-java-mods-note',
		defaultMessage:
			'Disabling removes the launcher patches. The lwjgl3ify and UniMixins mods stay in the instance — remove them from the Mods tab if you no longer want them.',
	},
	javaInstallation: {
		id: 'instance.settings.tabs.java.java-installation',
		defaultMessage: 'Java installation',
	},
	customJavaInstallation: {
		id: 'instance.settings.tabs.java.custom-java-installation',
		defaultMessage: 'Custom Java installation',
	},
	javaPathPlaceholder: {
		id: 'instance.settings.tabs.java.java-path-placeholder',
		defaultMessage: '/path/to/java',
	},
	javaMemory: {
		id: 'instance.settings.tabs.java.java-memory',
		defaultMessage: 'Memory allocated',
	},
	customMemoryAllocation: {
		id: 'instance.settings.tabs.java.custom-memory-allocation',
		defaultMessage: 'Custom memory allocation',
	},
	javaArguments: {
		id: 'instance.settings.tabs.java.java-arguments',
		defaultMessage: 'Java arguments',
	},
	customJavaArguments: {
		id: 'instance.settings.tabs.java.custom-java-arguments',
		defaultMessage: 'Custom Java arguments',
	},
	enterJavaArguments: {
		id: 'instance.settings.tabs.java.enter-java-arguments',
		defaultMessage: 'Enter Java arguments...',
	},
	javaEnvironmentVariables: {
		id: 'instance.settings.tabs.java.environment-variables',
		defaultMessage: 'Environment variables',
	},
	customEnvironmentVariables: {
		id: 'instance.settings.tabs.java.custom-environment-variables',
		defaultMessage: 'Custom environment variables',
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
	<div>
		<JavaDetectionModal ref="javaDetectionModal" @submit="(val) => (javaPath = val.path)" />
		<template v-if="modernJava?.supported">
			<h2 class="m-0 mb-2 text-lg font-extrabold text-contrast block">
				{{ formatMessage(messages.modernJava) }}
			</h2>
			<div class="flex gap-3 items-start p-4 bg-bg rounded-2xl mb-4">
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
								version: modernJava.installed_version ?? '?',
								javaMajor: activeJavaMajor ?? modernJava.java_major ?? '?',
							})
						}}
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
		</template>
		<h2 class="m-0 mb-2 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaInstallation) }}
		</h2>
		<Checkbox
			v-model="overrideJavaInstall"
			:label="formatMessage(messages.customJavaInstallation)"
			class="mb-2"
		/>
		<div class="flex gap-4 p-4 bg-bg rounded-2xl">
			<div class="flex gap-3 items-start flex-1 min-w-0">
				<div
					class="w-10 h-10 flex items-center justify-center rounded-full bg-button-bg border-solid border-[1px] border-button-border p-2 mt-1 shrink-0 [&_svg]:h-full [&_svg]:w-full"
				>
					<CoffeeIcon />
				</div>
				<div class="flex flex-col gap-2 flex-1 min-w-0">
					<span class="font-semibold leading-none mt-2"
						>Java {{ optimalJava?.parsed_version }}</span
					>
					<div class="flex gap-2 items-center">
						<StyledInput
							:model-value="activePath"
							:disabled="!overrideJavaInstall"
							autocomplete="off"
							:placeholder="formatMessage(messages.javaPathPlaceholder)"
							wrapper-class="flex-1 min-w-0"
							@update:model-value="(val) => (javaPath = String(val))"
						/>
						<Button
							type="quiet"
							:color="
								!hoveringTest && !testingJava
									? javaTestResult === true
										? 'green'
										: 'red'
									: undefined
							"
							:disabled="!overrideJavaInstall || testingJava"
							:style="{
								'--legacy-button-color':
									(!hoveringTest && !testingJava
										? javaTestResult === true
											? 'green'
											: 'red'
										: 'standard') &&
									(!hoveringTest && !testingJava
										? javaTestResult === true
											? 'green'
											: 'red'
										: 'standard') !== 'standard'
										? `var(--color-${
												!hoveringTest && !testingJava
													? javaTestResult === true
														? 'green'
														: 'red'
													: 'standard'
											})`
										: undefined,
							}"
							class="!text-[var(--legacy-button-color,var(--color-base))] [&>svg]:!text-[var(--legacy-button-color,var(--color-primary))]"
							@click="testJavaInstallation(activePath, optimalJava?.parsed_version, true)"
							@mouseenter="overrideJavaInstall && (hoveringTest = true)"
							@mouseleave="hoveringTest = false"
						>
							<SpinnerIcon v-if="testingJava" class="animate-spin h-4 w-4" />
							<CheckCircleIcon
								v-else-if="javaTestResult === true && !hoveringTest"
								class="h-4 w-4"
							/>
							<XCircleIcon v-else-if="javaTestResult !== true && !hoveringTest" class="h-4 w-4" />
							<RefreshCwIcon v-else-if="overrideJavaInstall" class="h-4 w-4" />
						</Button>
					</div>
					<div v-if="overrideJavaInstall" class="flex gap-2">
						<Button @click="handleDetectJava">
							<SearchIcon />
							Detect
						</Button>
						<Button @click="handleBrowseJava">
							<FolderSearchIcon />
							Browse
						</Button>
					</div>
				</div>
			</div>
		</div>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaMemory) }}
		</h2>
		<Checkbox
			v-model="overrideMemorySettings"
			:label="formatMessage(messages.customMemoryAllocation)"
			class="mb-2"
		/>
		<Slider
			id="max-memory"
			v-model="memory.maximum"
			:disabled="!overrideMemorySettings"
			:min="512"
			:max="maxMemory"
			:step="64"
			:snap-points="snapPoints"
			:snap-range="512"
			unit="MB"
		/>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaArguments) }}
		</h2>
		<Checkbox
			v-model="overrideJavaArgs"
			:label="formatMessage(messages.customJavaArguments)"
			class="my-2"
		/>
		<StyledInput
			id="java-args"
			v-model="javaArgs"
			autocomplete="off"
			:disabled="!overrideJavaArgs"
			:placeholder="formatMessage(messages.enterJavaArguments)"
			wrapper-class="w-full"
		/>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaEnvironmentVariables) }}
		</h2>
		<Checkbox
			v-model="overrideEnvVars"
			:label="formatMessage(messages.customEnvironmentVariables)"
			class="mb-2"
		/>
		<StyledInput
			id="env-vars"
			v-model="envVars"
			autocomplete="off"
			:disabled="!overrideEnvVars"
			:placeholder="formatMessage(messages.enterEnvironmentVariables)"
			wrapper-class="w-full"
		/>
	</div>
</template>
