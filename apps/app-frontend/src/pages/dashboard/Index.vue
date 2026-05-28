<script setup lang="ts">
import { defineMessages, NavTabs, useVIntl } from '@modrinth/ui'
import { useRoute } from 'vue-router'

import { useBreadcrumbs } from '@/store/breadcrumbs.js'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	tabCollections: { id: 'dashboard.tab.collections', defaultMessage: 'Collections' },
	tabNotifications: { id: 'dashboard.tab.notifications', defaultMessage: 'Notifications' },
})

breadcrumbs.setRootContext({ name: 'Dashboard', link: route.path })
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
