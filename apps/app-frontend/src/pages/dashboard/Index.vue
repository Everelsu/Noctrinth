<script setup lang="ts">
import { UserIcon } from '@modrinth/assets'
import { commonMessages, useVIntl } from '@modrinth/ui'
import { useRoute } from 'vue-router'

import { useRootBreadcrumb } from '@/providers/breadcrumbs'

const { formatMessage } = useVIntl()
const route = useRoute()

useRootBreadcrumb({
	slot: 'root',
	id: 'dashboard',
	label: () => formatMessage(commonMessages.dashboardLabel),
	to: '/dashboard',
	visual: { type: 'icon', component: UserIcon },
})
</script>

<template>
	<div class="p-6 flex flex-col gap-3">
		<h1 class="m-0 text-2xl hidden">{{ formatMessage(commonMessages.dashboardLabel) }}</h1>
		<RouterView v-if="route.path.startsWith('/dashboard')" />
	</div>
</template>
