<script setup>
import { CoffeeIcon } from '@modrinth/assets'
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import JavaSelector from '@/components/ui/JavaSelector.vue'
import JavaGpuPreference from '@/components/ui/settings/instances/JavaGpuPreference.vue'
import JavaRuntimeManager from '@/components/ui/settings/instances/JavaRuntimeManager.vue'
import { get_java_versions, set_java_version } from '@/helpers/jre'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	javaLocation: {
		id: 'app.settings.java-installations.location.title',
		defaultMessage: 'Java {version, number} location',
	},
})

const javaVersions = ref(await get_java_versions().catch(handleError))

// Deleting a runtime can invalidate a configured path, and the selectors above
// each hold their own copy — so re-read them, and remount the list so its own
// `await` runs again.
const runtimeManagerKey = ref(0)
async function reloadJavaVersions() {
	javaVersions.value = (await get_java_versions().catch(handleError)) ?? javaVersions.value
	runtimeManagerKey.value += 1
}

// Majors the launcher is expected to need. Minecraft asks for whichever one its
// own manifest names, so this is a floor rather than the whole truth — anything
// already configured is folded in below, which is how a newly installed major
// (26, say) appears here without this list being edited again.
const KNOWN_JAVA_MAJORS = [26, 25, 21, 17, 8]

const shownJavaMajors = computed(() => {
	const configured = Object.keys(javaVersions.value ?? {})
		.map(Number)
		.filter(Number.isFinite)

	return [...new Set([...KNOWN_JAVA_MAJORS, ...configured])].sort((a, b) => b - a)
})
async function updateJavaVersion(version) {
	if (version?.path === '') {
		version.path = undefined
	}

	if (version?.path) {
		version.path = version.path.replace('java.exe', 'javaw.exe')
	}

	await set_java_version(version).catch(handleError)
}
</script>
<template>
	<div class="flex flex-col gap-4">
		<div
			v-for="javaVersion in shownJavaMajors"
			:key="`java-${javaVersion}`"
			class="flex items-start gap-3 rounded-2xl bg-bg p-4"
		>
			<div
				class="mt-1 flex h-10 w-10 shrink-0 items-center justify-center rounded-full border-[1px] border-solid border-button-border bg-button-bg p-2 [&_svg]:h-full [&_svg]:w-full"
			>
				<CoffeeIcon />
			</div>
			<div class="flex min-w-0 flex-1 flex-col gap-2">
				<h2 class="m-0 text-lg font-extrabold leading-none text-contrast">
					{{ formatMessage(messages.javaLocation, { version: javaVersion }) }}
				</h2>
				<JavaSelector
					:id="'java-selector-' + javaVersion"
					v-model="javaVersions[javaVersion]"
					:version="javaVersion"
					@update:model-value="updateJavaVersion"
				/>
				<JavaGpuPreference :path="javaVersions[javaVersion]?.path ?? ''" />
			</div>
		</div>

		<hr class="my-2 bg-button-border border-none h-[1px]" />

		<Suspense>
			<JavaRuntimeManager :key="runtimeManagerKey" @changed="reloadJavaVersions" />
		</Suspense>
	</div>
</template>
