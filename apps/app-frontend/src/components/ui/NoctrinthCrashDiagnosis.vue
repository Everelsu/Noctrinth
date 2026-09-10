<script setup lang="ts">
/**
 * What the launcher makes of the files a crashed run left behind.
 *
 * The rules live in Rust — see `crash_analysis.rs` — and report an id and what
 * they captured. The words are here, so that they can be translated and so that
 * a fix can be a button rather than a sentence telling somebody where to look.
 */
import { ExternalIcon, FolderOpenIcon, SettingsIcon, WrenchIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	Collapsible,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { CrashFinding, CrashSeverity } from '@/helpers/noctrinth-crash'
import { showInstanceInFolder } from '@/helpers/utils.js'

const props = defineProps<{
	instanceId: string
	findings: CrashFinding[]
	/** Opens the instance's settings, on the tab a fix belongs to. */
	onOpenSettings?: (tab?: number) => void
}>()

const emit = defineEmits<{ dismiss: [] }>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

/**
 * The settings tab Java and memory live on.
 *
 * Upstream moved them into the overrides tab in 0.20.0; a fix that opens the
 * wrong tab is worse than one that opens none, so this is checked against
 * `settings-modal/index.vue` when that page changes.
 */
const JAVA_SETTINGS_TAB = 2

const messages = defineMessages({
	header: {
		id: 'app.crash.header',
		defaultMessage: 'The launcher found something in this crash',
	},
	headerNote: {
		id: 'app.crash.header-note',
		defaultMessage: 'Worth knowing about this run',
	},
	showEvidence: { id: 'app.crash.show-evidence', defaultMessage: 'Show the line' },
	hideEvidence: { id: 'app.crash.hide-evidence', defaultMessage: 'Hide the line' },
	openSettings: { id: 'app.crash.open-settings', defaultMessage: 'Open settings' },
	openFolder: { id: 'app.crash.open-folder', defaultMessage: 'Open instance folder' },
	dismiss: { id: 'app.crash.dismiss', defaultMessage: 'Dismiss' },
	foundIn: { id: 'app.crash.found-in', defaultMessage: 'In {file}' },

	// One title and one fix per rule. A rule with nothing useful to suggest has
	// no fix line rather than a filler one.
	crash_description: {
		id: 'app.crash.rule.crash-description.title',
		defaultMessage: 'The game described it as: {description}',
	},
	loader_suggestion: {
		id: 'app.crash.rule.loader-suggestion.title',
		defaultMessage: 'The mod loader worked out a fix itself',
	},
	loader_suggestion_fix: {
		id: 'app.crash.rule.loader-suggestion.fix',
		defaultMessage: '{suggestion}',
	},
	suspected_mods: {
		id: 'app.crash.rule.suspected-mods.title',
		defaultMessage: 'The mod loader suspects: {mods}',
	},
	suspected_mods_fix: {
		id: 'app.crash.rule.suspected-mods.fix',
		defaultMessage: 'Turn those off first, one at a time, and launch again after each.',
	},
	out_of_memory_heap: {
		id: 'app.crash.rule.out-of-memory-heap.title',
		defaultMessage: 'The game ran out of the memory it was given',
	},
	out_of_memory_heap_fix: {
		id: 'app.crash.rule.out-of-memory-heap.fix',
		defaultMessage:
			'Give this instance more memory, or ask less of it: a lower render distance and fewer world-generation mods are what usually costs the most.',
	},
	out_of_memory_metaspace: {
		id: 'app.crash.rule.out-of-memory-metaspace.title',
		defaultMessage: 'The game ran out of room for the code it was loading',
	},
	out_of_memory_metaspace_fix: {
		id: 'app.crash.rule.out-of-memory-metaspace.fix',
		defaultMessage:
			'This is a very large modpack on a small allocation. Raise the memory for this instance.',
	},
	out_of_memory_system: {
		id: 'app.crash.rule.out-of-memory-system.title',
		defaultMessage: 'The machine had no memory left to give',
	},
	out_of_memory_system_fix: {
		id: 'app.crash.rule.out-of-memory-system.fix',
		defaultMessage:
			'Lower the memory this instance asks for, or close what else is running. Asking for more than the machine has is what stops it starting at all.',
	},
	heap_too_large_to_start: {
		id: 'app.crash.rule.heap-too-large-to-start.title',
		defaultMessage: 'The instance asked for more memory than could be reserved ({size})',
	},
	heap_too_large_to_start_fix: {
		id: 'app.crash.rule.heap-too-large-to-start.fix',
		defaultMessage: 'Lower the allocation for this instance and launch again.',
	},
	java_too_old: {
		id: 'app.crash.rule.java-too-old.title',
		defaultMessage: 'Something needs Java {needs}, and the instance ran on Java {has}',
	},
	java_too_old_fix: {
		id: 'app.crash.rule.java-too-old.fix',
		defaultMessage: 'Point this instance at a Java {needs} installation, or let it download one.',
	},
	java_unsupported_class: {
		id: 'app.crash.rule.java-unsupported-class.title',
		defaultMessage: 'A mod was built for a newer Java than the one that ran',
	},
	java_unsupported_class_fix: {
		id: 'app.crash.rule.java-unsupported-class.fix',
		defaultMessage: 'Change the Java this instance uses in its settings.',
	},
	fabric_missing_dependency: {
		id: 'app.crash.rule.fabric-missing-dependency.title',
		defaultMessage: '{mod_name} needs {dependency}, which is not installed',
	},
	fabric_missing_dependency_fix: {
		id: 'app.crash.rule.fabric-missing-dependency.fix',
		defaultMessage: 'Install {dependency}, or remove {mod_name}.',
	},
	fabric_wrong_dependency_version: {
		id: 'app.crash.rule.fabric-wrong-dependency-version.title',
		defaultMessage: '{mod_name} needs another version of {dependency}',
	},
	fabric_wrong_dependency_version_fix: {
		id: 'app.crash.rule.fabric-wrong-dependency-version.fix',
		defaultMessage: 'Update {dependency} to {requirement}, or use a build of {mod_name} that fits.',
	},
	forge_missing_dependency: {
		id: 'app.crash.rule.forge-missing-dependency.title',
		defaultMessage: '{mod_id} needs {dependency}, which is missing',
	},
	forge_missing_dependency_fix: {
		id: 'app.crash.rule.forge-missing-dependency.fix',
		defaultMessage: 'Install {dependency}, or remove {mod_id}.',
	},
	duplicate_mods: {
		id: 'app.crash.rule.duplicate-mods.title',
		defaultMessage: 'The same mod is installed more than once',
	},
	duplicate_mods_fix: {
		id: 'app.crash.rule.duplicate-mods.fix',
		defaultMessage:
			'Open the mods folder and leave one copy of each — two versions of the same file is the usual cause.',
	},
	mod_for_other_version: {
		id: 'app.crash.rule.mod-for-other-version.title',
		defaultMessage: 'A mod was built for a different version of Minecraft',
	},
	mod_for_other_version_fix: {
		id: 'app.crash.rule.mod-for-other-version.fix',
		defaultMessage:
			'It went looking for {symbol}, which this version does not have. Update the mod, or use the version of it built for this one.',
	},
	mixin_failed: {
		id: 'app.crash.rule.mixin-failed.title',
		defaultMessage: 'A mod could not patch the game ({config})',
	},
	mixin_failed_fix: {
		id: 'app.crash.rule.mixin-failed.fix',
		defaultMessage:
			'The name in front of .mixins.json is the mod to update or remove first; when two mods patch the same thing, it is the pair that has to change.',
	},
	optifine_present: {
		id: 'app.crash.rule.optifine-present.title',
		defaultMessage: 'OptiFine was loaded when this went wrong',
	},
	optifine_present_fix: {
		id: 'app.crash.rule.optifine-present.fix',
		defaultMessage:
			'OptiFine breaks against most modern mods. Sodium with Iris, or Embeddium with Oculus, do the same job and are made to sit beside them.',
	},
	connector_fabric_mod: {
		id: 'app.crash.rule.connector-fabric-mod.title',
		defaultMessage: 'A Fabric rendering mod was loaded through Sinytra Connector',
	},
	connector_fabric_mod_fix: {
		id: 'app.crash.rule.connector-fabric-mod.fix',
		defaultMessage:
			'Connector cannot carry Sodium, Iris or Indium. Use the Forge builds — Embeddium and Oculus — instead.',
	},
	corrupted_archive: {
		id: 'app.crash.rule.corrupted-archive.title',
		defaultMessage: 'A file the game had to read is damaged or half-downloaded',
	},
	corrupted_archive_fix: {
		id: 'app.crash.rule.corrupted-archive.fix',
		defaultMessage:
			'Repair the instance so the launcher fetches it again. A download that was interrupted leaves exactly this.',
	},
	disk_full: {
		id: 'app.crash.rule.disk-full.title',
		defaultMessage: 'The disk is full',
	},
	disk_full_fix: {
		id: 'app.crash.rule.disk-full.fix',
		defaultMessage: 'Free some space on the drive the instance is on and launch again.',
	},
	file_locked: {
		id: 'app.crash.rule.file-locked.title',
		defaultMessage: 'Another program was holding one of the game’s files',
	},
	file_locked_fix: {
		id: 'app.crash.rule.file-locked.fix',
		defaultMessage:
			'Close other launchers and any copy of the game still running, and check whether an antivirus is scanning this folder.',
	},
	gpu_driver_amd: {
		id: 'app.crash.rule.gpu-driver-amd.title',
		defaultMessage: 'The AMD graphics driver crashed ({library})',
	},
	gpu_driver_nvidia: {
		id: 'app.crash.rule.gpu-driver-nvidia.title',
		defaultMessage: 'The NVIDIA graphics driver crashed ({library})',
	},
	gpu_driver_intel: {
		id: 'app.crash.rule.gpu-driver-intel.title',
		defaultMessage: 'The Intel graphics driver crashed ({library})',
	},
	gpu_driver_fix: {
		id: 'app.crash.rule.gpu-driver.fix',
		defaultMessage:
			'Update the graphics driver from the maker’s site rather than through Windows Update, and turn off shaders while testing.',
	},
	opengl_unsupported: {
		id: 'app.crash.rule.opengl-unsupported.title',
		defaultMessage: 'The graphics driver could not give the game the OpenGL it needs',
	},
	opengl_unsupported_fix: {
		id: 'app.crash.rule.opengl-unsupported.fix',
		defaultMessage:
			'Update the graphics driver. On a laptop with two GPUs, make sure the game is set to run on the faster one.',
	},
	missing_native_library: {
		id: 'app.crash.rule.missing-native-library.title',
		defaultMessage: 'A library the game loads from disk could not be loaded',
	},
	missing_native_library_fix: {
		id: 'app.crash.rule.missing-native-library.fix',
		defaultMessage:
			'Repair the instance. If it happens again, check that an antivirus is not quarantining the natives folder.',
	},
	jvm_problematic_frame: {
		id: 'app.crash.rule.jvm-problematic-frame.title',
		defaultMessage: 'Java itself stopped in {frame}',
	},
	jvm_signal: {
		id: 'app.crash.rule.jvm-signal.title',
		defaultMessage: 'The process was stopped by {signal}',
	},
})

/** Rules whose fix is a page of this launcher rather than a sentence. */
const ACTIONS: Record<string, 'settings' | 'folder'> = {
	out_of_memory_heap: 'settings',
	out_of_memory_metaspace: 'settings',
	out_of_memory_system: 'settings',
	heap_too_large_to_start: 'settings',
	java_too_old: 'settings',
	java_unsupported_class: 'settings',
	duplicate_mods: 'folder',
	corrupted_archive: 'folder',
	missing_native_library: 'folder',
	file_locked: 'folder',
}

/** A class file version is its Java version plus forty-four. */
function javaOf(classVersion: string | undefined): string {
	const version = Number(classVersion)
	return Number.isFinite(version) && version > 44 ? String(version - 44) : (classVersion ?? '?')
}

function values(finding: CrashFinding): Record<string, string> {
	const filled = { ...finding.values }

	if (finding.rule === 'java_too_old') {
		filled.needs = javaOf(finding.values.class_version)
		filled.has = javaOf(finding.values.runtime_version)
	}

	return filled
}

function messageFor(key: string, finding: CrashFinding): string | undefined {
	const message = (messages as Record<string, { id: string; defaultMessage: string }>)[key]
	if (!message) return undefined

	return formatMessage(message, values(finding))
}

function titleOf(finding: CrashFinding): string {
	return messageFor(finding.rule, finding) ?? finding.evidence
}

function fixOf(finding: CrashFinding): string | undefined {
	if (finding.rule.startsWith('gpu_driver_')) return messageFor('gpu_driver_fix', finding)
	return messageFor(`${finding.rule}_fix`, finding)
}

const worst = computed<CrashSeverity>(() =>
	props.findings.some((finding) => finding.severity === 'critical')
		? 'critical'
		: props.findings.some((finding) => finding.severity === 'warning')
			? 'warning'
			: 'note',
)

const admonitionType = computed(() =>
	worst.value === 'critical' ? 'critical' : worst.value === 'warning' ? 'warning' : 'info',
)

const shownEvidence = ref<string | null>(null)

function toggleEvidence(rule: string) {
	shownEvidence.value = shownEvidence.value === rule ? null : rule
}

function runAction(finding: CrashFinding) {
	const action = ACTIONS[finding.rule]
	if (action === 'settings') {
		props.onOpenSettings?.(JAVA_SETTINGS_TAB)
	} else if (action === 'folder') {
		void showInstanceInFolder(props.instanceId).catch(handleError)
	}
}
</script>

<template>
	<Admonition
		v-if="findings.length"
		:type="admonitionType"
		:header="formatMessage(worst === 'note' ? messages.headerNote : messages.header)"
	>
		<div class="flex flex-col gap-3">
			<div v-for="finding in findings" :key="finding.rule" class="flex flex-col gap-1">
				<span class="font-semibold text-contrast">{{ titleOf(finding) }}</span>
				<span v-if="fixOf(finding)" class="text-secondary">{{ fixOf(finding) }}</span>

				<div class="flex flex-wrap items-center gap-2 pt-1">
					<Button v-if="ACTIONS[finding.rule] === 'settings'" size="sm" @click="runAction(finding)">
						<SettingsIcon aria-hidden="true" />
						{{ formatMessage(messages.openSettings) }}
					</Button>
					<Button v-if="ACTIONS[finding.rule] === 'folder'" size="sm" @click="runAction(finding)">
						<FolderOpenIcon aria-hidden="true" />
						{{ formatMessage(messages.openFolder) }}
					</Button>
					<Button size="sm" type="transparent" @click="toggleEvidence(finding.rule)">
						<WrenchIcon aria-hidden="true" />
						{{
							formatMessage(
								shownEvidence === finding.rule ? messages.hideEvidence : messages.showEvidence,
							)
						}}
					</Button>
					<span class="text-sm text-secondary">
						{{ formatMessage(messages.foundIn, { file: finding.source_name }) }}
					</span>
				</div>

				<Collapsible :collapsed="shownEvidence !== finding.rule">
					<code
						class="mt-1 block overflow-x-auto whitespace-pre rounded-lg bg-surface-2 p-2 text-xs"
					>
						{{ finding.evidence }}
					</code>
				</Collapsible>
			</div>

			<div class="flex items-center gap-2">
				<Button size="sm" type="transparent" @click="emit('dismiss')">
					<ExternalIcon aria-hidden="true" class="rotate-90" />
					{{ formatMessage(messages.dismiss) }}
				</Button>
			</div>
		</div>
	</Admonition>
</template>
