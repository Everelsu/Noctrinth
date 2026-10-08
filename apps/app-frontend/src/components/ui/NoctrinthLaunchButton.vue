<template>
	<Motion
		as="div"
		class="relative h-12 shrink-0"
		:initial="false"
		:animate="{ width: mainWidth + (againVisible ? GAP + HEIGHT : 0) }"
		:transition="mainSpring"
	>
		<span
			ref="measure"
			class="pointer-events-none invisible absolute left-0 top-0 whitespace-nowrap text-base font-extrabold"
			aria-hidden="true"
			>{{ label }}</span
		>

		<!--
			The liquid: two blobs drawn through a goo filter (blur, then a hard
			alpha threshold), so where they come close they pull into one shape.
			The buttons themselves sit on top, transparent, so the text and icons
			stay sharp.
		-->
		<svg class="absolute size-0" aria-hidden="true">
			<filter :id="gooId" x="-20%" y="-50%" width="140%" height="200%">
				<feGaussianBlur in="SourceGraphic" stdDeviation="5" result="blur" />
				<feColorMatrix
					in="blur"
					mode="matrix"
					values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7"
					result="goo"
				/>
				<feComposite in="SourceGraphic" in2="goo" operator="atop" />
			</filter>
		</svg>
		<div
			class="pointer-events-none absolute inset-0 overflow-visible"
			:style="{ filter: `url(#${gooId})` }"
			aria-hidden="true"
		>
			<Motion
				as="div"
				class="absolute inset-y-0 left-0 rounded-2xl"
				:initial="false"
				:animate="{ width: mainWidth, backgroundColor: colors.main }"
				:transition="mainSpring"
			/>
			<Motion
				as="div"
				class="absolute left-0 top-0 size-12 rounded-full"
				:initial="false"
				:animate="{
					x: againVisible ? mainWidth + GAP : mainWidth - HEIGHT,
					scale: againVisible ? 1 : 0.55,
					backgroundColor: colors.again,
				}"
				:transition="liquidSpring"
			/>
		</div>

		<Motion
			as="button"
			type="button"
			class="launch-face absolute inset-y-0 left-0 flex items-center justify-center overflow-hidden rounded-2xl border-0 bg-transparent p-0 text-base font-extrabold"
			:class="busy ? 'cursor-default' : 'cursor-pointer'"
			:style="{ color: colors.text }"
			:initial="false"
			:animate="{ width: mainWidth }"
			:transition="mainSpring"
			:disabled="busy"
			:aria-label="label"
			@click="onMainClick"
		>
			<Transition name="nm-morph">
				<span :key="state" class="flex items-center gap-2 whitespace-nowrap">
					<LoaderSpinnerIcon v-if="busy" class="size-6 shrink-0 motion-safe:animate-spin" />
					<StopCircleIcon v-else-if="state === 'stop'" class="size-6 shrink-0" />
					<PlayIcon v-else class="size-6 shrink-0" />
					{{ label }}
				</span>
			</Transition>
		</Motion>

		<Motion
			v-tooltip="againVisible ? labels.again : undefined"
			as="button"
			type="button"
			class="launch-face absolute left-0 top-0 grid size-12 place-items-center rounded-full border-0 bg-transparent p-0"
			:class="againDisabled ? 'cursor-default' : 'cursor-pointer'"
			:style="{ color: colors.text, pointerEvents: againVisible ? 'auto' : 'none' }"
			:initial="false"
			:animate="{
				x: againVisible ? mainWidth + GAP : mainWidth - HEIGHT,
				opacity: againVisible ? 1 : 0,
				scale: againVisible ? 1 : 0.55,
			}"
			:transition="liquidSpring"
			:disabled="againDisabled || !againVisible"
			:aria-label="labels.again"
			:aria-hidden="!againVisible"
			:tabindex="againVisible ? 0 : -1"
			@click="emit('again')"
		>
			<PlayIcon class="size-6" />
		</Motion>
	</Motion>
</template>

<script setup lang="ts">
/**
 * The instance's launch button as one shape that flows between its states:
 * Play, Starting, Installing, Stop and Stopping change its width and colour on a
 * spring rather than swapping one button for another, and the label rises and
 * falls through a blur. Starting a second copy is a drop that runs out of the
 * Stop button and back into it.
 */
import { LoaderSpinnerIcon, PlayIcon, StopCircleIcon } from '@modrinth/assets'
import { Motion } from 'motion-v'
import { computed, onBeforeUnmount, onMounted, ref, useId, useTemplateRef, watch } from 'vue'

export type LaunchState = 'play' | 'starting' | 'installing' | 'stop' | 'stopping'

const props = defineProps<{
	state: LaunchState
	againVisible: boolean
	againDisabled?: boolean
	labels: Record<LaunchState | 'again', string>
}>()

const emit = defineEmits<{ play: []; stop: []; again: [] }>()

const HEIGHT = 48
/** Wider than the goo reaches at rest, so the drop comes away once it has run out. */
const GAP = 14
/** Horizontal padding, the icon and the gap after it, around the measured label. */
const CHROME = 14 * 2 + 24 + 8

const reducedMotion =
	typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
const mainSpring = reducedMotion
	? { duration: 0 }
	: { type: 'spring' as const, stiffness: 380, damping: 32, mass: 0.9 }
/** Looser, so the drop overshoots and settles like something liquid. */
const liquidSpring = reducedMotion
	? { duration: 0 }
	: { type: 'spring' as const, stiffness: 240, damping: 17, mass: 0.9 }

const gooId = `nm-launch-goo-${useId()}`
const label = computed(() => props.labels[props.state])
const busy = computed(
	() => props.state === 'starting' || props.state === 'installing' || props.state === 'stopping',
)

const measure = useTemplateRef<HTMLSpanElement>('measure')
const mainWidth = ref(140)
function remeasure() {
	if (measure.value) mainWidth.value = Math.ceil(measure.value.offsetWidth) + CHROME
}
watch(label, remeasure, { flush: 'post' })

/** Motion animates numbers and colours, not CSS variables, so the theme's are read out. */
const themeTick = ref(0)
const colors = computed(() => {
	void themeTick.value
	const styles = getComputedStyle(document.documentElement)
	const read = (name: string, fallback: string) => styles.getPropertyValue(name).trim() || fallback
	const brand = read('--color-brand', '#bd85ff')
	const red = read('--color-red', '#ff496e')
	const stopping = props.state === 'stop' || props.state === 'stopping'
	return {
		main: stopping ? red : brand,
		again: brand,
		text: read('--color-accent-contrast', '#000000'),
	}
})

let observer: MutationObserver | undefined
let resizeObserver: ResizeObserver | undefined
onMounted(() => {
	remeasure()
	// Fonts arriving late change the label's width.
	resizeObserver = new ResizeObserver(remeasure)
	if (measure.value) resizeObserver.observe(measure.value)
	observer = new MutationObserver(() => themeTick.value++)
	observer.observe(document.documentElement, {
		attributes: true,
		attributeFilter: ['class', 'style'],
	})
})
onBeforeUnmount(() => {
	observer?.disconnect()
	resizeObserver?.disconnect()
})

function onMainClick() {
	if (busy.value) return
	if (props.state === 'stop') emit('stop')
	else emit('play')
}
</script>

<style scoped>
.launch-face {
	transition: background-color 150ms ease-out;
}
.launch-face:not(:disabled):hover {
	background-color: rgb(255 255 255 / 0.1);
}
.launch-face:not(:disabled):active {
	background-color: rgb(0 0 0 / 0.08);
}
.launch-face:focus-visible {
	outline: none;
	box-shadow: 0 0 0 4px var(--color-brand-shadow, rgb(189 133 255 / 0.35));
}
</style>
