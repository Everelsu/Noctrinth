<script setup lang="ts">
import { NavTabs } from '@modrinth/ui'
import { useRoute } from 'vue-router'

import { useBreadcrumbs } from '@/store/breadcrumbs.js'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Dashboard', link: route.path })
</script>

<template>
	<div class="p-6 flex flex-col gap-3">
		<h1 class="m-0 text-2xl hidden">Dashboard</h1>
		<NavTabs
			:links="[
				{
					label: 'Collections',
					href: `/dashboard/collections`,
					subpages: ['/collection/'],
				},
				{ label: 'Notifications', href: `/dashboard/notifications` },
			]"
		/>
		<RouterView v-if="route.path.startsWith('/dashboard')" />
	</div>
</template>
