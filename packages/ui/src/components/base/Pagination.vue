<template>
	<div v-if="count > 1" class="flex items-center gap-1">
		<template v-if="page > 1">
			<ButtonLink
				v-if="linkFunction"
				aria-label="Previous Page"
				:href="linkFunction(page - 1)"
				type="quiet"
				class="!w-9 !px-0 !rounded-full"
				@click.prevent="switchPage(page - 1)"
			>
				<ChevronLeftIcon aria-hidden="true" />
			</ButtonLink>
			<IconButton v-else label="Previous Page" type="quiet" @click="switchPage(page - 1)">
				<ChevronLeftIcon aria-hidden="true" />
			</IconButton>
		</template>
		<div
			v-for="(item, index) in pages"
			:key="'page-' + item + '-' + index"
			:class="{
				'page-number': page !== item,
				shrink: item !== '-' && item > 99,
			}"
			class="page-number-container"
		>
			<template v-if="item === '-'">
				<!-- Clickable gap: shows an inline page-number input on click. -->
				<input
					v-if="editingGap === index"
					ref="gapInputRef"
					v-model="gapInputValue"
					type="number"
					:min="1"
					:max="count"
					class="page-number-input"
					:aria-label="`Jump to page (1 to ${count})`"
					@keydown.enter="submitGap"
					@keydown.escape="cancelGap"
					@blur="cancelGap"
				/>
				<IconButton
					v-else
					type="quiet"
					class="rotate-90"
					:label="`Jump to page (1 to ${count})`"
					:title="`Jump to page (1–${count})`"
					@click="openGap(index)"
				>
					<EllipsisVerticalIcon />
				</IconButton>
			</template>
			<template v-else>
				<ButtonLink
					v-if="linkFunction"
					:href="linkFunction(item)"
					type="quiet"
					:color="page === item ? 'brand' : undefined"
					:interaction="page === item ? 'filled' : undefined"
					:aria-current="page === item ? 'page' : undefined"
					:class="['!min-w-9 !rounded-full', page === item ? '!bg-brand-highlight' : '']"
					@click.prevent="page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</ButtonLink>
				<Button
					v-else
					type="quiet"
					:color="page === item ? 'brand' : undefined"
					:interaction="page === item ? 'filled' : undefined"
					:aria-current="page === item ? 'page' : undefined"
					:class="['!min-w-9 !rounded-full', page === item ? '!bg-brand-highlight' : '']"
					@click="page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</Button>
			</template>
		</div>

		<template v-if="page !== pages[pages.length - 1]">
			<ButtonLink
				v-if="linkFunction"
				aria-label="Next Page"
				:href="linkFunction(page + 1)"
				type="quiet"
				class="!w-9 !px-0 !rounded-full"
				@click.prevent="switchPage(page + 1)"
			>
				<ChevronRightIcon aria-hidden="true" />
			</ButtonLink>
			<IconButton v-else label="Next Page" type="quiet" @click="switchPage(page + 1)">
				<ChevronRightIcon aria-hidden="true" />
			</IconButton>
		</template>
	</div>
</template>
<script setup lang="ts">
import { ChevronLeftIcon, ChevronRightIcon, EllipsisVerticalIcon } from '@modrinth/assets'
import { computed, nextTick, ref } from 'vue'

import { Button, ButtonLink, IconButton } from './buttons'

const emit = defineEmits<{
	'switch-page': [page: number]
}>()

const props = withDefaults(
	defineProps<{
		page: number
		count: number
		linkFunction?: (page: number) => string | undefined
	}>(),
	{
		page: 1,
		count: 1,
	},
)

const editingGap = ref<number | null>(null)
const gapInputValue = ref<string>('')
const gapInputRef = ref<HTMLInputElement | null>(null)

function openGap(index: number) {
	editingGap.value = index
	gapInputValue.value = ''
	nextTick(() => gapInputRef.value?.focus())
}

function submitGap() {
	const n = Number(gapInputValue.value)
	if (Number.isFinite(n) && n >= 1 && n <= props.count) {
		editingGap.value = null
		switchPage(Math.floor(n))
	} else {
		cancelGap()
	}
}

function cancelGap() {
	editingGap.value = null
	gapInputValue.value = ''
}

const pages = computed(() => {
	const pages: ('-' | number)[] = []

	const first = 1
	const last = props.count
	const current = props.page
	const prev = current - 1
	const next = current + 1
	const gap = '-'

	if (prev > first) {
		pages.push(first)
	}
	if (prev > first + 1) {
		pages.push(gap)
	}
	if (prev >= first) {
		pages.push(prev)
	}
	pages.push(current)
	if (next <= last) {
		pages.push(next)
	}
	if (next < last - 1) {
		pages.push(gap)
	}
	if (next < last) {
		pages.push(last)
	}

	return pages
})

function switchPage(newPage: number) {
	emit('switch-page', Math.min(Math.max(newPage, 1), props.count))
}
</script>

<style scoped>
.page-number-input {
	width: 4rem;
	height: 2rem;
	padding: 0 0.5rem;
	border-radius: 9999px;
	border: 1px solid var(--color-button-bg);
	background: var(--color-raised-bg);
	color: var(--color-contrast);
	text-align: center;
	font-size: 0.875rem;
	outline: none;
}
.page-number-input:focus {
	border-color: var(--color-brand);
}
/* Hide the spin buttons — they crowd the small input. */
.page-number-input::-webkit-outer-spin-button,
.page-number-input::-webkit-inner-spin-button {
	-webkit-appearance: none;
	margin: 0;
}
.page-number-input[type='number'] {
	-moz-appearance: textfield;
}
</style>
