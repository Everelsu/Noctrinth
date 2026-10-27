import { type ContentActionWarning, type ContentItem, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, type Ref } from 'vue'

import type { GameInstance } from '@/helpers/types'

const managedSourceKinds = new Set([
	'shared_instance',
	'modrinth_modpack',
	'imported_modpack',
	// Noctrinth's own: what a CurseForge pack's own files are stamped with.
	'curseforge_modpack',
])

/**
 * Whether an item belongs to what manages the instance — a shared instance's
 * owner, or a quarantined instance as a whole — and so is not the user's to change.
 */
export function isManagedContentIn(instance: GameInstance, item: ContentItem): boolean {
	return (
		instance.quarantined ||
		(instance.shared_instance?.role === 'member' &&
			!!item.source_kind &&
			managedSourceKinds.has(item.source_kind))
	)
}

/** Whether an item can be updated in place, outside the instance's own content tab too. */
export function canUpdateContentIn(instance: GameInstance, item: ContentItem): boolean {
	return (
		!isManagedContentIn(instance, item) &&
		!!item.file_path &&
		!!item.has_update &&
		!!item.update_version_id
	)
}

export function useManagedContentPolicy(instance: Ref<GameInstance>) {
	const { formatMessage } = useVIntl()
	const isManagedModpack = computed(() => instance.value.shared_instance?.role === 'member')
	const isQuarantined = computed(() => instance.value.quarantined)
	const canUnpublish = computed(() => instance.value.shared_instance?.role === 'owner')
	const canUnlink = computed(() => instance.value.shared_instance?.role === 'member')
	const managedModpackWarning = computed(() => ({
		admonitionHeader: formatMessage(messages.warningHeader),
		changeVersionBody: formatMessage(messages.changeVersionBody),
		unlinkBody: formatMessage(messages.unlinkBody),
	}))

	function isManagedContent(item: ContentItem) {
		return isManagedContentIn(instance.value, item)
	}

	function canMutateContent(item: ContentItem) {
		return !isManagedContent(item)
	}

	function canUpdateContent(item: ContentItem) {
		return canUpdateContentIn(instance.value, item)
	}

	function deleteWarning(items: ContentItem[]): ContentActionWarning | null {
		if (!items.some(isManagedContent)) return null
		return {
			admonitionHeader: formatMessage(messages.warningHeader),
			admonitionBody: formatMessage(
				items.length === 1 ? messages.deleteSingleBody : messages.deleteBulkBody,
			),
			actionLabel: formatMessage(
				items.length === 1 ? messages.deleteButton : messages.deleteManyButton,
				{ count: items.length },
			),
		}
	}

	function disableWarning(items: ContentItem[]): ContentActionWarning | null {
		if (!items.some(isManagedContent)) return null
		return {
			admonitionHeader: formatMessage(messages.warningHeader),
			admonitionBody: formatMessage(
				items.length === 1 ? messages.disableSingleBody : messages.disableBulkBody,
			),
			actionLabel: formatMessage(
				items.length === 1 ? messages.disableButton : messages.disableManyButton,
				{ count: items.length },
			),
		}
	}

	return {
		isManagedModpack,
		isQuarantined,
		canUnpublish,
		canUnlink,
		managedModpackWarning,
		isManagedContent,
		canMutateContent,
		canUpdateContent,
		deleteWarning,
		disableWarning,
	}
}

const messages = defineMessages({
	warningHeader: {
		id: 'content.shared-instance.warning-header',
		defaultMessage: 'This is part of the shared instance',
	},
	changeVersionBody: {
		id: 'content.shared-instance.change-version-body',
		defaultMessage:
			'Changing the version only changes your local copy. Future shared instance updates may restore or change it again.',
	},
	unlinkBody: {
		id: 'content.shared-instance.unlink-body',
		defaultMessage:
			'Unlinking only changes your local copy. Future shared instance updates may restore or change it again.',
	},
	deleteSingleBody: {
		id: 'content.shared-instance.delete-single-body',
		defaultMessage:
			'Deleting it only changes your local copy. Future shared instance updates may restore or change it again.',
	},
	deleteBulkBody: {
		id: 'content.shared-instance.delete-bulk-body',
		defaultMessage:
			'Some selected projects are part of the shared instance. Deleting them only changes your local copy, and future shared instance updates may restore or change them again.',
	},
	deleteButton: { id: 'content.shared-instance.delete-button', defaultMessage: 'Delete anyway' },
	deleteManyButton: {
		id: 'content.shared-instance.delete-many-button',
		defaultMessage: 'Delete {count, number} projects anyway',
	},
	disableSingleBody: {
		id: 'content.shared-instance.disable-single-body',
		defaultMessage:
			'Disabling it only changes your local copy. Future shared instance updates may re-enable, restore, or change it again.',
	},
	disableBulkBody: {
		id: 'content.shared-instance.disable-bulk-body',
		defaultMessage:
			'Some selected projects are part of the shared instance. Disabling them only changes your local copy, and future shared instance updates may re-enable, restore, or change them again.',
	},
	disableButton: { id: 'content.shared-instance.disable-button', defaultMessage: 'Disable anyway' },
	disableManyButton: {
		id: 'content.shared-instance.disable-many-button',
		defaultMessage: 'Disable {count, number} projects anyway',
	},
})
