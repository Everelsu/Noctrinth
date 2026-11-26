<template>
	<Card>
		<NoctrinthTranslatableDescription
			:description="project.body"
			:fallback-description="fallbackDescription"
		/>
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
	fallbackDescription: String,
})
</script>

<script>
export default {
	name: 'Description',
}
</script>
