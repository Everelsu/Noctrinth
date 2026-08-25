<script setup lang="ts">
import { UserIcon } from '@modrinth/assets'
import { useRoute } from 'vue-router'

import { useRootBreadcrumb } from '@/providers/breadcrumbs'

const route = useRoute()

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
		<RouterView v-if="route.path.startsWith('/dashboard')" />
	</div>
</template>
