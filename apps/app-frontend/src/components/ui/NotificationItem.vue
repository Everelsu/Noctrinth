<template>
	<div
		class="notification"
		:class="{
			'has-body': hasBody,
			read: notification.read,
		}"
	>
		<RouterLink v-if="!type" :to="notification.link || '#'" class="notification__icon backed-svg">
			<BellIcon />
		</RouterLink>
		<DoubleIcon v-else class="notification__icon">
			<template #primary>
				<RouterLink v-if="project" :to="projectLink(project)" tabindex="-1">
					<Avatar size="xs" :src="project.icon_url" no-shadow />
				</RouterLink>
				<a v-else-if="organization" href="#" tabindex="-1" @click.prevent="openOrg(organization)">
					<Avatar size="xs" :src="organization.icon_url" no-shadow />
				</a>
				<a v-else-if="user" href="#" tabindex="-1" @click.prevent="openUser(user)">
					<Avatar size="xs" :src="user.avatar_url" no-shadow />
				</a>
				<Avatar v-else size="xs" no-shadow />
			</template>
			<template #secondary>
				<ScaleIcon
					v-if="type === 'moderator_message' || type === 'status_change'"
					class="moderation-color"
				/>
				<UserPlusIcon v-else-if="type === 'team_invite' && project" class="creator-color" />
				<UserPlusIcon
					v-else-if="type === 'organization_invite' && organization"
					class="creator-color"
				/>
				<VersionIcon v-else-if="type === 'project_update' && project && version" />
				<BellIcon v-else />
			</template>
		</DoubleIcon>

		<div class="notification__title">
			<template v-if="type === 'project_update' && project && version">
				<IntlFormatted :message-id="messages.projectUpdated" :values="{ title: project.title }">
					<template #project="{ children }">
						<RouterLink :to="projectLink(project)" class="title-link">
							<component :is="() => children" />
						</RouterLink>
					</template>
				</IntlFormatted>
			</template>
			<template v-else-if="type === 'team_invite' && project">
				<a
					v-if="invitedBy"
					href="#"
					class="title-link inline-flex items-center gap-1"
					@click.prevent="openUser(invitedBy)"
				>
					<Avatar :src="invitedBy.avatar_url" circle size="xxs" no-shadow />
					<span>{{ invitedBy.username }}</span>
				</a>
				<span>
					<IntlFormatted :message-id="messages.invitedToJoin" :values="{ name: project.title }">
						<template #target="{ children }">
							<RouterLink :to="projectLink(project)" class="title-link">
								<component :is="() => children" />
							</RouterLink>
						</template>
					</IntlFormatted>
				</span>
			</template>
			<template v-else-if="type === 'organization_invite' && organization">
				<a
					v-if="invitedBy"
					href="#"
					class="title-link inline-flex items-center gap-1"
					@click.prevent="openUser(invitedBy)"
				>
					<Avatar :src="invitedBy.avatar_url" circle size="xxs" no-shadow />
					<span>{{ invitedBy.username }}</span>
				</a>
				<span>
					<IntlFormatted :message-id="messages.invitedToJoin" :values="{ name: organization.name }">
						<template #target="{ children }">
							<a href="#" class="title-link" @click.prevent="openOrg(organization)">
								<component :is="() => children" />
							</a>
						</template>
					</IntlFormatted>
				</span>
			</template>
			<template v-else-if="type === 'status_change' && project">
				<IntlFormatted
					:message-id="messages.statusChanged"
					:values="{
						title: project.title,
						oldStatus: notification.body?.old_status,
						newStatus: notification.body?.new_status,
					}"
				>
					<template #project="{ children }">
						<RouterLink :to="projectLink(project)" class="title-link">
							<component :is="() => children" />
						</RouterLink>
					</template>
					<template #strong="{ children }">
						<strong><component :is="() => children" /></strong>
					</template>
				</IntlFormatted>
			</template>
			<template v-else-if="type === 'moderator_message' && project">
				<IntlFormatted
					:message-id="messages.moderatorMessage"
					:values="{ title: project.title, count: notification.grouped_notifs ? 2 : 1 }"
				>
					<template #project="{ children }">
						<RouterLink :to="projectLink(project)" class="title-link">
							<component :is="() => children" />
						</RouterLink>
					</template>
				</IntlFormatted>
			</template>
			<a v-else href="#" class="title-link" @click.prevent="openExternal(notification.link)">
				{{ notification.title }}
			</a>
		</div>

		<div v-if="hasBody" class="notification__body">
			<div v-if="type === 'project_update'" class="version-list">
				<div v-for="notif in groupedWithVersion" :key="notif.id" class="version-link">
					<VersionIcon />
					<RouterLink
						:to="versionLink(notif.extra_data!.project, notif.extra_data!.version)"
						class="text-link"
					>
						{{ notif.extra_data!.version.name }}
					</RouterLink>
					<span class="version-info">
						{{ formatMessage(messages.versionFor) }}
						<span class="loaders">{{ formatLoaders(notif.extra_data!.version) }}</span>
						{{ formatVersions(notif.extra_data!.version.game_versions) }}
						<span :title="formatDateTime(notif.extra_data!.version.date_published)" class="date">
							{{ formatRelative(notif.extra_data!.version.date_published) }}
						</span>
					</span>
				</div>
			</div>
			<template v-else>
				{{ notification.text }}
			</template>
		</div>

		<span class="notification__date">
			<span v-if="notification.read" class="read-badge inline-flex items-center gap-1">
				<CheckCircleIcon /> {{ formatMessage(messages.readBadge) }}
			</span>
			<span :title="formatDateTime(notification.created)" class="inline-flex items-center gap-1">
				<CalendarIcon />
				{{ formatMessage(messages.received, { time: formatRelative(notification.created) }) }}
			</span>
		</span>

		<div class="notification__actions">
			<div class="input-group">
				<template
					v-if="(type === 'team_invite' || type === 'organization_invite') && !notification.read"
				>
					<ButtonStyled color="brand">
						<button @click="onAccept">
							<CheckIcon />
							{{ formatMessage(messages.accept) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="red">
						<button @click="onDecline">
							<XIcon />
							{{ formatMessage(messages.decline) }}
						</button>
					</ButtonStyled>
				</template>
				<ButtonStyled v-else-if="!notification.read">
					<button @click="onMarkRead">
						<CheckIcon />
						{{ formatMessage(messages.markAsRead) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	BellIcon,
	CalendarIcon,
	CheckCircleIcon,
	CheckIcon,
	ScaleIcon,
	UserPlusIcon,
	VersionIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	defineMessages,
	DoubleIcon,
	injectNotificationManager,
	IntlFormatted,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed } from 'vue'

import { get as getCreds } from '@/helpers/mr_auth.ts'
import {
	acceptTeamInvite,
	declineTeamInvite,
	deleteIds,
	markIdsAsRead,
	type PlatformNotification,
} from '@/helpers/platform-notifications'

dayjs.extend(relativeTime)

const props = defineProps<{
	notification: PlatformNotification
}>()

const emit = defineEmits<{
	'update:notification': []
	read: [string[]]
	remove: [string[]]
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	projectUpdated: {
		id: 'app.notifications.project-updated',
		defaultMessage: 'A project you follow, <project>{title}</project>, has been updated:',
	},
	invitedToJoin: {
		id: 'app.notifications.invited-to-join',
		defaultMessage: 'has invited you to join <target>{name}</target>.',
	},
	statusChanged: {
		id: 'app.notifications.status-changed',
		defaultMessage:
			'<project>{title}</project> updated from <strong>{oldStatus}</strong> to <strong>{newStatus}</strong> by the moderators.',
	},
	moderatorMessage: {
		id: 'app.notifications.moderator-message',
		defaultMessage:
			'Your project, <project>{title}</project>, has received {count, plural, one {a message} other {messages}} from the moderators.',
	},
	versionFor: {
		id: 'app.notifications.version-for',
		defaultMessage: 'for',
	},
	readBadge: {
		id: 'app.notifications.read-badge',
		defaultMessage: 'Read',
	},
	received: {
		id: 'app.notifications.received',
		defaultMessage: 'Received {time}',
	},
	accept: {
		id: 'app.notifications.accept',
		defaultMessage: 'Accept',
	},
	decline: {
		id: 'app.notifications.decline',
		defaultMessage: 'Decline',
	},
	markAsRead: {
		id: 'app.notifications.mark-as-read',
		defaultMessage: 'Mark as read',
	},
	errorMarkRead: {
		id: 'app.notifications.error.mark-read',
		defaultMessage: 'Error marking notification as read',
	},
	errorAccept: {
		id: 'app.notifications.error.accept',
		defaultMessage: 'Error accepting invite',
	},
	errorDecline: {
		id: 'app.notifications.error.decline',
		defaultMessage: 'Error declining invite',
	},
	notSignedIn: {
		id: 'app.notifications.not-signed-in',
		defaultMessage: 'Not signed in',
	},
})

const type = computed(() =>
	!props.notification.body || props.notification.body.type === 'legacy_markdown'
		? null
		: props.notification.body.type,
)
const project = computed(() => props.notification.extra_data?.project)
const version = computed(() => props.notification.extra_data?.version)
const user = computed(() => props.notification.extra_data?.user)
const organization = computed(() => props.notification.extra_data?.organization)
const invitedBy = computed(() => props.notification.extra_data?.invited_by)

const hasBody = computed(() => !type.value || type.value === 'project_update')

const groupedWithVersion = computed(() => {
	const all = props.notification.grouped_notifs
		? [props.notification, ...props.notification.grouped_notifs]
		: [props.notification]
	return all.filter((x) => x.extra_data?.version)
})

interface NotifEntity {
	id?: string
	slug?: string
	username?: string
	loaders?: string[]
}

function projectLink(p: NotifEntity): string {
	return `/project/${p.slug || p.id}`
}
function versionLink(p: NotifEntity, v: NotifEntity): string {
	return `/project/${p.slug || p.id}/version/${v.id}`
}
function openUser(u: NotifEntity) {
	openUrl(`https://modrinth.com/user/${u.username}`)
}
function openOrg(o: NotifEntity) {
	openUrl(`https://modrinth.com/organization/${o.slug || o.id}`)
}
function openExternal(link?: string) {
	if (!link) return
	if (link.startsWith('/')) {
		openUrl(`https://modrinth.com${link}`)
	} else {
		openUrl(link)
	}
}

function formatRelative(date: string): string {
	return dayjs(date).fromNow()
}
function formatDateTime(date: string): string {
	return dayjs(date).format('MMMM D, YYYY [at] h:mm A')
}
function formatLoaders(v: NotifEntity): string {
	return (v?.loaders || []).map((l: string) => l.charAt(0).toUpperCase() + l.slice(1)).join(', ')
}
function formatVersions(versions: string[] | undefined): string {
	if (!versions || versions.length === 0) return ''
	if (versions.length === 1) return versions[0]
	return `${versions[0]} – ${versions[versions.length - 1]}`
}

function idsForThis(): string[] {
	return [props.notification.id, ...(props.notification.grouped_notifs?.map((n) => n.id) || [])]
}

async function onMarkRead() {
	const ids = idsForThis()
	// Optimistic — flip locally first so the UI feels instant.
	emit('read', ids)
	try {
		await markIdsAsRead(ids)
	} catch (err) {
		addNotification({
			title: formatMessage(messages.errorMarkRead),
			text: (err as Error).message,
			type: 'error',
		})
		emit('update:notification')
	}
}

async function onAccept() {
	const teamId = props.notification.body?.team_id
	if (!teamId) return
	const ids = idsForThis()
	try {
		await acceptTeamInvite(teamId)
		emit('read', ids)
		markIdsAsRead(ids).catch(() => {})
	} catch (err) {
		addNotification({
			title: formatMessage(messages.errorAccept),
			text: (err as Error).message,
			type: 'error',
		})
	}
}

async function onDecline() {
	const teamId = props.notification.body?.team_id
	if (!teamId) return
	const ids = idsForThis()
	try {
		const creds = await getCreds()
		if (!creds) throw new Error(formatMessage(messages.notSignedIn))
		await declineTeamInvite(teamId, creds.user_id)
		emit('remove', ids)
		deleteIds(ids).catch(() => {})
	} catch (err) {
		addNotification({
			title: formatMessage(messages.errorDecline),
			text: (err as Error).message,
			type: 'error',
		})
	}
}
</script>

<style lang="scss" scoped>
.notification {
	display: grid;
	grid-template:
		'icon title'
		'actions actions'
		'date date';
	grid-template-columns: min-content 1fr;
	grid-template-rows: min-content min-content min-content;
	gap: var(--gap-sm);

	&.has-body {
		grid-template:
			'icon title'
			'body body'
			'actions actions'
			'date date';
		grid-template-columns: min-content 1fr;
		grid-template-rows: min-content auto auto min-content;
	}

	.notification__icon {
		grid-area: icon;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.notification__title {
		grid-area: title;
		color: var(--color-contrast);
		margin-block: auto;
		display: inline-block;
		vertical-align: middle;
		line-height: 1.25rem;
	}

	.notification__body {
		grid-area: body;

		.version-list {
			margin: 0;
			padding: 0;
			list-style-type: none;
			display: flex;
			flex-direction: column;
			gap: var(--gap-sm);

			.version-link {
				display: flex;
				flex-direction: row;
				gap: var(--gap-xs);
				align-items: center;
				flex-wrap: wrap;

				.version-info {
					display: contents;

					.loaders {
						color: var(--color-contrast);
					}

					.date {
						color: var(--color-secondary);
						font-size: var(--font-size-sm);
					}
				}
			}
		}
	}

	.notification__date {
		grid-area: date;
		color: var(--color-secondary);
		font-size: var(--font-size-sm);
		display: flex;
		align-items: center;
		gap: var(--gap-sm);

		svg {
			vertical-align: top;
		}

		.read-badge {
			font-weight: bold;
			color: var(--color-contrast);
		}
	}

	.notification__actions {
		grid-area: actions;
		display: flex;
		flex-direction: row;
		gap: var(--gap-sm);

		.input-group {
			display: flex;
			gap: var(--gap-sm);
		}
	}

	.title-link {
		font-weight: bold;
		color: var(--color-link, var(--color-brand));
		text-decoration: none;

		&:hover {
			text-decoration: underline;
		}
	}

	.text-link {
		color: var(--color-link, var(--color-brand));
		text-decoration: none;

		&:hover {
			text-decoration: underline;
		}
	}

	.moderation-color {
		color: var(--color-orange);
	}

	.creator-color {
		color: var(--color-blue);
	}
}
</style>
