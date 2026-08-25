<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { ServerStackIcon } from '@modrinth/assets'
import {
	defineMessages,
	injectModrinthClient,
	ServersManagePageIndex,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { useRootBreadcrumb } from '@/providers/breadcrumbs'

import { config } from '../config'

const messages = defineMessages({
	breadcrumb: { id: 'app.hosting.title', defaultMessage: 'Hosting' },
})

const { formatMessage } = useVIntl()

const stripePublishableKey = (config.stripePublishableKey as string) || ''

const client = injectModrinthClient()

useRootBreadcrumb({
	slot: 'root',
	id: 'servers',
	label: () => formatMessage(messages.breadcrumb),
	to: '/hosting/manage/',
	visual: { type: 'icon', component: ServerStackIcon },
})

const { data: products } = useQuery({
	queryKey: ['billing', 'products'],
	queryFn: () => client.labrinth.billing_internal.getProducts(),
})

const resolvedProducts = computed<Labrinth.Billing.Internal.Product[]>(() => products.value ?? [])
</script>

<template>
	<ServersManagePageIndex
		:stripe-publishable-key="stripePublishableKey"
		:products="resolvedProducts"
	/>
</template>
