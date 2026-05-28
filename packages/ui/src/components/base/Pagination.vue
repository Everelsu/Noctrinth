<template>
	<div v-if="count > 1" class="flex items-center gap-1">
		<ButtonStyled v-if="page > 1" circular type="transparent">
			<a
				v-if="linkFunction"
				aria-label="Previous Page"
				:href="linkFunction(page - 1)"
				@click.prevent="switchPage(page - 1)"
			>
				<ChevronLeftIcon />
			</a>
			<button v-else aria-label="Previous Page" @click="switchPage(page - 1)">
				<ChevronLeftIcon />
			</button>
		</ButtonStyled>
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
				<button
					v-else
					type="button"
					class="page-gap-button rotate-90 grid place-content-center"
					:aria-label="`Jump to page (1 to ${count})`"
					:title="`Jump to page (1–${count})`"
					@click="openGap(index)"
				>
					<EllipsisVerticalIcon />
				</button>
			</template>
			<ButtonStyled
				v-else
				circular
				:color="page === item ? 'brand' : 'standard'"
				:type="page === item ? 'highlight' : 'transparent'"
			>
				<a
					v-if="linkFunction"
					:href="linkFunction(item)"
					:class="page === item ? '!text-brand' : ''"
					@click.prevent="page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</a>
				<button
					v-else
					:class="page === item ? '!text-brand' : ''"
					@click="page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</button>
			</ButtonStyled>
		</div>

		<ButtonStyled v-if="page !== pages[pages.length - 1]" circular type="transparent">
			<a
				v-if="linkFunction"
				aria-label="Next Page"
				:href="linkFunction(page + 1)"
				@click.prevent="switchPage(page + 1)"
			>
				<ChevronRightIcon />
			</a>
			<button v-else aria-label="Next Page" @click="switchPage(page + 1)">
				<ChevronRightIcon />
			</button>
		</ButtonStyled>
	</div>
</template>
<script setup lang="ts">
import { ChevronLeftIcon, ChevronRightIcon, EllipsisVerticalIcon } from '@modrinth/assets'
import { computed, nextTick, ref } from 'vue'

import ButtonStyled from './ButtonStyled.vue'

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
		linkFunction: (page: number) => void page,
	},
)

// ── Gap-click page input ──────────────────────────────────────────────────
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
.page-gap-button {
	background: transparent;
	border: none;
	padding: 0.25rem;
	color: inherit;
	cursor: pointer;
	border-radius: 9999px;
	transition: background-color 120ms ease;
}
.page-gap-button:hover {
	background: var(--color-button-bg);
}
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
