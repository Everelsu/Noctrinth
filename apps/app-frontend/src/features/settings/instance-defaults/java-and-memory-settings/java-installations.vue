<script setup lang="ts">
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'

import JavaSelector from '@/features/java/java-selector/index.vue'
import { javaKeys, javaVersionsQueryOptions } from '@/features/java/queries'
import { setJavaVersion } from '@/platform/app-lib/jre/commands'
import type { JavaVersion } from '@/platform/app-lib/jre/types'

import JavaGpuPreference from './JavaGpuPreference.vue'
import JavaRuntimeManager from './JavaRuntimeManager.vue'

const JAVA_MAJOR_VERSIONS = [26, 25, 21, 17, 8]

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const queryClient = useQueryClient()

const { data: javaVersions } = useQuery(javaVersionsQueryOptions())

const javaVersionMutation = useMutation({
	mutationFn: ({ selection }: { majorVersion: number; selection: Partial<JavaVersion> }) =>
		setJavaVersion(selection),
	networkMode: 'always',
	onMutate: ({ majorVersion, selection }) => {
		queryClient.setQueryData(javaVersionsQueryOptions().queryKey, (current) => ({
			...current,
			[majorVersion]: selection as JavaVersion,
		}))
	},
	onError: handleError,
	onSettled: () => queryClient.invalidateQueries({ queryKey: javaKeys.versions }),
})

function updateJavaVersion(majorVersion: number, selection: Partial<JavaVersion>) {
	javaVersionMutation.mutate({
		majorVersion,
		selection: {
			...selection,
			path: selection.path ? selection.path.replace('java.exe', 'javaw.exe') : undefined,
		},
	})
}

const shownJavaMajors = computed(() =>
	[
		...new Set([
			...JAVA_MAJOR_VERSIONS,
			...Object.keys(javaVersions.value ?? {})
				.map(Number)
				.filter(Number.isFinite),
		]),
	].sort((a, b) => b - a),
)
const runtimeManagerKey = ref(0)
async function reloadJavaVersions() {
	await queryClient.invalidateQueries({ queryKey: javaKeys.versions })
	runtimeManagerKey.value += 1
}
const messages = defineMessages({
	javaLocation: {
		id: 'app.settings.java-installations.location.title',
		defaultMessage: 'Java {version, number}',
	},
})
</script>

<template>
	<div class="flex flex-col gap-4">
		<div
			v-for="majorVersion in shownJavaMajors"
			:key="`java-${majorVersion}`"
			class="flex flex-col gap-2.5"
		>
			<h3 class="m-0 text-base font-semibold text-contrast">
				{{ formatMessage(messages.javaLocation, { version: majorVersion }) }}
			</h3>
			<JavaSelector
				:id="`java-selector-${majorVersion}`"
				:model-value="javaVersions?.[majorVersion]"
				:version="majorVersion"
				@update:model-value="(selection) => updateJavaVersion(majorVersion, selection)"
			>
				<template #footer>
					<JavaGpuPreference :path="javaVersions?.[majorVersion]?.path ?? ''" />
				</template>
			</JavaSelector>
		</div>
		<Suspense
			><JavaRuntimeManager :key="runtimeManagerKey" @changed="reloadJavaVersions"
		/></Suspense>
	</div>
</template>
