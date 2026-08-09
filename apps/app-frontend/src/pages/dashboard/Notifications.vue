<script setup lang="ts">
import { CheckCheckIcon, HistoryIcon } from '@modrinth/assets'
import {
	Button,
	Chips,
	defineMessages,
	injectNotificationManager,
	Pagination,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import NotificationItem from '@/components/ui/NotificationItem.vue'
import { getUserNotifications } from '@/helpers/modrinth-api'
import { get as getCreds } from '@/helpers/mr_auth.ts'
import {
	fetchExtraNotificationData,
	groupNotifications,
	markIdsAsRead,
	patchCachedNotifications,
	type PlatformNotification,
	type RawNotification,
	readNotificationsCache,
	writeNotificationsCache,
} from '@/helpers/platform-notifications'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	headingNotifications: { id: 'notifications.heading', defaultMessage: 'Notifications' },
	headingHistory: { id: 'notifications.heading-history', defaultMessage: 'Notification history' },
	viewHistory: { id: 'notifications.view-history', defaultMessage: 'View history' },
	markAllRead: { id: 'notifications.mark-all-read', defaultMessage: 'Mark all as read' },
	back: { id: 'notifications.back', defaultMessage: 'Back' },
	loading: { id: 'notifications.loading', defaultMessage: 'Loading notifications...' },
	signInPrompt: {
		id: 'notifications.sign-in-prompt',
		defaultMessage: 'Sign in to view notifications',
	},
	signInPromptBody: {
		id: 'notifications.sign-in-prompt-body',
		defaultMessage: 'Sign in to your Modrinth account to see your notifications here.',
	},
	historyEmpty: {
		id: 'notifications.history-empty',
		defaultMessage: 'No notifications in history.',
	},
	unreadEmpty: {
		id: 'notifications.unread-empty',
		defaultMessage: "You don't have any unread notifications.",
	},
	typeAll: { id: 'notifications.type.all', defaultMessage: 'All' },
	typeProjectUpdate: { id: 'notifications.type.project-update', defaultMessage: 'Updates' },
	typeModeratorMessage: {
		id: 'notifications.type.moderator-message',
		defaultMessage: 'Moderator messages',
	},
	typeStatusChange: { id: 'notifications.type.status-change', defaultMessage: 'Status changes' },
	typeTeamInvite: { id: 'notifications.type.team-invite', defaultMessage: 'Team invites' },
	typeOrganizationInvite: {
		id: 'notifications.type.organization-invite',
		defaultMessage: 'Organization invites',
	},
	typeOther: { id: 'notifications.type.other', defaultMessage: 'Other' },
})

const signedIn = ref(true)
const rawNotifications = ref<PlatformNotification[]>([])
const showHistory = ref(false)
const selectedType = ref('all')
const page = ref(1)
const perPage = 50

/**
 * Fetch fresh notifications from the API, enrich, store in cache.
 * `silent` skips the wait-for-completion semantics — used for the background
 * stale-while-revalidate refresh after a cache hit.
 */
async function fetchFresh(userId: string): Promise<PlatformNotification[]> {
	const fetched = (await getUserNotifications(userId)) as RawNotification[]
	fetched.sort((a, b) => new Date(b.created).getTime() - new Date(a.created).getTime())
	const enriched = await fetchExtraNotificationData(fetched as PlatformNotification[])
	writeNotificationsCache(userId, enriched)
	return enriched
}

// Reload helper kept for `@update:notification` events from list items.
async function load() {
	try {
		const creds = await getCreds()
		if (!creds) {
			signedIn.value = false
			rawNotifications.value = []
			return
		}
		signedIn.value = true
		rawNotifications.value = await fetchFresh(creds.user_id)
	} catch (e) {
		handleError(e)
		rawNotifications.value = []
	}
}

// Initial render: use the per-session cache if present so the page is instant,
// then refresh in the background. Only block <Suspense> on the network when
// the cache is cold.
try {
	const creds = await getCreds()
	if (!creds) {
		signedIn.value = false
	} else {
		signedIn.value = true
		const cached = readNotificationsCache(creds.user_id)
		if (cached) {
			rawNotifications.value = cached
			// Background refresh — failures don't disturb the user.
			fetchFresh(creds.user_id)
				.then((fresh) => {
					rawNotifications.value = fresh
				})
				.catch(() => {})
		} else {
			rawNotifications.value = await fetchFresh(creds.user_id)
		}
	}
} catch (e) {
	handleError(e)
}

const visibleByReadState = computed(() =>
	rawNotifications.value.filter((n) => (showHistory.value ? n.read : !n.read)),
)

const notifTypes = computed(() => {
	const types = [...new Set(visibleByReadState.value.map((n) => n.body?.type || 'other'))]
	return types.length > 1 ? ['all', ...types] : types
})

const filtered = computed(() =>
	visibleByReadState.value.filter(
		(n) => selectedType.value === 'all' || (n.body?.type || 'other') === selectedType.value,
	),
)

const pages = computed(() => Math.max(1, Math.ceil(filtered.value.length / perPage)))

const paginated = computed(() => {
	const start = (page.value - 1) * perPage
	return groupNotifications(filtered.value.slice(start, start + perPage))
})

const hasRead = computed(() => rawNotifications.value.some((n) => n.read))

function toggleHistory() {
	showHistory.value = !showHistory.value
	selectedType.value = 'all'
	page.value = 1
}

async function getUserId(): Promise<string | null> {
	const creds = await getCreds()
	return creds?.user_id ?? null
}

async function readAll() {
	const ids: string[] = []
	for (const n of paginated.value) {
		ids.push(n.id)
		if (n.grouped_notifs) ids.push(...n.grouped_notifs.map((g) => g.id))
	}
	// Optimistic
	const idSet = new Set(ids)
	rawNotifications.value = rawNotifications.value.map((n) =>
		idSet.has(n.id) ? { ...n, read: true } : n,
	)
	const uid = await getUserId()
	if (uid) {
		patchCachedNotifications(uid, (n) => (idSet.has(n.id) ? { ...n, read: true } : n))
	}
	try {
		await markIdsAsRead(ids)
	} catch (e) {
		handleError(e)
	}
}

async function onRead(ids: string[]) {
	const idSet = new Set(ids)
	rawNotifications.value = rawNotifications.value.map((n) =>
		idSet.has(n.id) ? { ...n, read: true } : n,
	)
	const uid = await getUserId()
	if (uid) {
		patchCachedNotifications(uid, (n) => (idSet.has(n.id) ? { ...n, read: true } : n))
	}
}

async function onRemove(ids: string[]) {
	const idSet = new Set(ids)
	rawNotifications.value = rawNotifications.value.filter((n) => !idSet.has(n.id))
	const uid = await getUserId()
	if (uid) {
		patchCachedNotifications(uid, (n) => (idSet.has(n.id) ? null : n))
	}
}

function changePage(newPage: number) {
	page.value = newPage
}

function formatType(t: string) {
	if (t === 'all') return formatMessage(messages.typeAll)
	const map: Record<string, string> = {
		project_update: formatMessage(messages.typeProjectUpdate),
		moderator_message: formatMessage(messages.typeModeratorMessage),
		status_change: formatMessage(messages.typeStatusChange),
		team_invite: formatMessage(messages.typeTeamInvite),
		organization_invite: formatMessage(messages.typeOrganizationInvite),
		legacy_markdown: formatMessage(messages.typeOther),
		other: formatMessage(messages.typeOther),
	}
	return map[t] || t.replace(/_/g, ' ')
}
</script>

<template>
	<div>
		<section class="universal-card">
			<div class="header__row">
				<div class="header__title">
					<h2 class="text-2xl">
						{{
							formatMessage(showHistory ? messages.headingHistory : messages.headingNotifications)
						}}
					</h2>
				</div>
				<template v-if="!showHistory">
					<Button v-if="hasRead" @click="toggleHistory">
						<HistoryIcon />
						{{ formatMessage(messages.viewHistory) }}
					</Button>
					<Button v-if="paginated.length > 0" type="colored" color="red" @click="readAll">
						<CheckCheckIcon />
						{{ formatMessage(messages.markAllRead) }}
					</Button>
				</template>
				<Button v-else @click="toggleHistory">
					{{ formatMessage(messages.back) }}
				</Button>
			</div>

			<Chips
				v-if="notifTypes.length > 1"
				v-model="selectedType"
				:items="notifTypes"
				:format-label="formatType"
				:capitalize="false"
			/>

			<div v-if="!signedIn" class="py-12 text-center">
				<p class="mt-4 text-lg font-medium text-contrast">
					{{ formatMessage(messages.signInPrompt) }}
				</p>
				<p class="text-sm text-secondary">
					{{ formatMessage(messages.signInPromptBody) }}
				</p>
			</div>

			<template v-else-if="paginated.length > 0">
				<NotificationItem
					v-for="n in paginated"
					:key="n.id"
					:notification="n"
					class="universal-card recessed"
					@read="onRead"
					@remove="onRemove"
					@update:notification="load"
				/>
			</template>

			<p v-else>
				{{ formatMessage(showHistory ? messages.historyEmpty : messages.unreadEmpty) }}
			</p>

			<div v-if="pages > 1" class="flex justify-end">
				<Pagination :page="page" :count="pages" @switch-page="changePage" />
			</div>
		</section>
	</div>
</template>

<style lang="scss" scoped>
.universal-card {
	padding: var(--gap-lg);
	background-color: var(--color-bg-raised);
	border-radius: var(--radius-lg);
	margin-bottom: var(--gap-md);

	h2 {
		margin: 0;
		color: var(--color-contrast);
	}

	&.recessed {
		background-color: var(--color-bg-raised);
		box-shadow: none;
		padding: var(--gap-md);
		border: 1px solid var(--color-button-border);
	}
}

.header__row {
	display: flex;
	align-items: center;
	gap: var(--gap-sm);
	margin-bottom: var(--gap-md);
	flex-wrap: wrap;
}

.header__title {
	flex: 1 1 auto;
	min-width: 0;

	h2 {
		margin: 0;
	}
}
</style>
