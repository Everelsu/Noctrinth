<script setup lang="ts">
import { UserIcon } from '@modrinth/assets'
import { defineMessages, NavTabs, useVIntl } from '@modrinth/ui'
import { useRoute } from 'vue-router'

import { useRootBreadcrumb } from '@/providers/breadcrumbs'

const route = useRoute()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	tabCollections: { id: 'dashboard.tab.collections', defaultMessage: 'Collections' },
	tabNotifications: { id: 'dashboard.tab.notifications', defaultMessage: 'Notifications' },
})

useRootBreadcrumb({
	slot: 'root',
	id: 'dashboard',
	label: 'Dashboard',
	to: '/dashboard',
	visual: { type: 'icon', component: UserIcon },
})
</script>

<template>
	<div class="p-6 flex flex-col gap-3">
		<h1 class="m-0 text-2xl hidden">Dashboard</h1>
		<NavTabs
			:links="[
				{
					label: formatMessage(messages.tabCollections),
					href: `/dashboard/collections`,
					subpages: ['/collection/'],
				},
				{ label: formatMessage(messages.tabNotifications), href: `/dashboard/notifications` },
			]"
		/>
		<RouterView v-if="route.path.startsWith('/dashboard')" />
	</div>
</template>
