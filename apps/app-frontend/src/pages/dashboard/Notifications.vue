<script setup lang="ts">
import { CheckCheckIcon, HistoryIcon } from '@modrinth/assets'
import { ButtonStyled, Chips, injectNotificationManager, Pagination } from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

import NotificationItem from '@/components/ui/NotificationItem.vue'
import { getUserNotifications } from '@/helpers/modrinth-api'
import { get as getCreds } from '@/helpers/mr_auth.ts'
import {
	fetchExtraNotificationData,
	groupNotifications,
	markIdsAsRead,
	type PlatformNotification,
	type RawNotification,
} from '@/helpers/platform-notifications'

const { handleError } = injectNotificationManager()

const loading = ref(true)
const signedIn = ref(true)
const rawNotifications = ref<PlatformNotification[]>([])
const showHistory = ref(false)
const selectedType = ref('all')
const page = ref(1)
const perPage = 50

async function load() {
	loading.value = true
	try {
		const creds = await getCreds()
		if (!creds) {
			signedIn.value = false
			rawNotifications.value = []
			return
		}
		signedIn.value = true
		const fetched = (await getUserNotifications(creds.user_id)) as RawNotification[]
		// Sort newest first
		fetched.sort((a, b) => new Date(b.created).getTime() - new Date(a.created).getTime())
		const enriched = await fetchExtraNotificationData(fetched as PlatformNotification[])
		rawNotifications.value = enriched
	} catch (e) {
		handleError(e)
		rawNotifications.value = []
	} finally {
		loading.value = false
	}
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
	try {
		await markIdsAsRead(ids)
	} catch (e) {
		handleError(e)
	}
}

function onRead(ids: string[]) {
	const idSet = new Set(ids)
	rawNotifications.value = rawNotifications.value.map((n) =>
		idSet.has(n.id) ? { ...n, read: true } : n,
	)
}

function onRemove(ids: string[]) {
	const idSet = new Set(ids)
	rawNotifications.value = rawNotifications.value.filter((n) => !idSet.has(n.id))
}

function changePage(newPage: number) {
	page.value = newPage
}

function formatType(t: string) {
	if (t === 'all') return 'All'
	const map: Record<string, string> = {
		project_update: 'Updates',
		moderator_message: 'Moderator messages',
		status_change: 'Status changes',
		team_invite: 'Team invites',
		organization_invite: 'Organization invites',
		legacy_markdown: 'Other',
		other: 'Other',
	}
	return map[t] || t.replace(/_/g, ' ')
}

onMounted(load)
</script>

<template>
	<div>
		<section class="universal-card">
			<div class="header__row">
				<div class="header__title">
					<h2 class="text-2xl">{{ showHistory ? 'Notification history' : 'Notifications' }}</h2>
				</div>
				<template v-if="!showHistory">
					<ButtonStyled v-if="hasRead">
						<button @click="toggleHistory">
							<HistoryIcon />
							View history
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="paginated.length > 0" color="red">
						<button @click="readAll">
							<CheckCheckIcon />
							Mark all as read
						</button>
					</ButtonStyled>
				</template>
				<ButtonStyled v-else>
					<button @click="toggleHistory">Back</button>
				</ButtonStyled>
			</div>

			<Chips
				v-if="notifTypes.length > 1"
				v-model="selectedType"
				:items="notifTypes"
				:format-label="formatType"
				:capitalize="false"
			/>

			<p v-if="loading">Loading notifications...</p>

			<div v-else-if="!signedIn" class="py-12 text-center">
				<p class="mt-4 text-lg font-medium text-contrast">Sign in to view notifications</p>
				<p class="text-sm text-secondary">
					Sign in to your Modrinth account to see your notifications here.
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
				{{
					showHistory
						? 'No notifications in history.'
						: "You don't have any unread notifications."
				}}
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
