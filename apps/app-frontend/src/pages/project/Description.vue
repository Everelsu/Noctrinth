<template>
	<Card>
		<NoctrinthTranslatableDescription :description="project.body" />
	</Card>
</template>

<script setup>
import { Card } from '@modrinth/ui'

import NoctrinthTranslatableDescription from '@/components/ui/NoctrinthTranslatableDescription.vue'

defineProps({
	project: {
		type: Object,
		default: () => {},
	},
})
</script>

<script>
export default {
	name: 'Description',
}
</script>
