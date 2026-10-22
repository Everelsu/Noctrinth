<script setup lang="ts">
/**
 * Noctrinth's own colour wheel: hue around the circle, saturation out from its
 * centre, brightness on a slider under it, and the hex value to type into.
 * Every change is emitted at once, so whatever listens can repaint as it moves.
 */
import { defineMessages, InputFrame, Slider, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

const model = defineModel<string>({ required: true })

const { formatMessage } = useVIntl()

const messages = defineMessages({
	wheel: {
		id: 'app.appearance-settings.accent.custom.wheel',
		defaultMessage: 'Hue and saturation',
	},
	brightness: {
		id: 'app.appearance-settings.accent.custom.brightness',
		defaultMessage: 'Brightness',
	},
	hex: {
		id: 'app.appearance-settings.accent.custom.hex',
		defaultMessage: 'Hex colour',
	},
})

function hsvToHex(hue: number, saturation: number, value: number): string {
	const channel = (n: number) => {
		const k = (n + hue / 60) % 6
		const level = value - value * saturation * Math.max(0, Math.min(k, 4 - k, 1))
		return Math.round(level * 255)
			.toString(16)
			.padStart(2, '0')
	}
	return `#${channel(5)}${channel(3)}${channel(1)}`
}

function hexToHsv(hex: string): [number, number, number] | null {
	const match = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
	if (!match) return null
	const number = Number.parseInt(match[1], 16)
	const [r, g, b] = [(number >> 16) & 255, (number >> 8) & 255, number & 255].map((c) => c / 255)
	const max = Math.max(r, g, b)
	const delta = max - Math.min(r, g, b)
	let hue = 0
	if (delta > 0) {
		if (max === r) hue = ((g - b) / delta) % 6
		else if (max === g) hue = (b - r) / delta + 2
		else hue = (r - g) / delta + 4
		hue = (hue * 60 + 360) % 360
	}
	return [hue, max === 0 ? 0 : delta / max, max]
}

const hue = ref(0)
const saturation = ref(1)
const value = ref(1)
const hexInput = ref('')

// Grey has no hue and black no saturation, so reading the model back would
// lose them; what this emitted itself is not read back.
let emitted = ''
watch(
	model,
	(color) => {
		hexInput.value = color.replace('#', '').toUpperCase()
		if (color.toLowerCase() === emitted) return
		const hsv = hexToHsv(color)
		if (!hsv) return
		if (hsv[1] > 0 && hsv[2] > 0) hue.value = hsv[0]
		if (hsv[2] > 0) saturation.value = hsv[1]
		value.value = hsv[2]
	},
	{ immediate: true },
)

function emit(): void {
	emitted = hsvToHex(hue.value, saturation.value, value.value)
	model.value = emitted
}

const wheel = ref<HTMLElement | null>(null)

function pickAt(event: PointerEvent): void {
	const rect = wheel.value?.getBoundingClientRect()
	if (!rect) return
	const radius = rect.width / 2
	const dx = event.clientX - rect.left - radius
	const dy = event.clientY - rect.top - radius
	hue.value = ((Math.atan2(dx, -dy) * 180) / Math.PI + 360) % 360
	saturation.value = Math.min(1, Math.hypot(dx, dy) / radius)
	emit()
}

function onPointerDown(event: PointerEvent): void {
	wheel.value?.setPointerCapture(event.pointerId)
	pickAt(event)
}

function onPointerMove(event: PointerEvent): void {
	if (wheel.value?.hasPointerCapture(event.pointerId)) pickAt(event)
}

function onKeydown(event: KeyboardEvent): void {
	const step = event.shiftKey ? 10 : 2
	if (event.key === 'ArrowLeft') hue.value = (hue.value - step + 360) % 360
	else if (event.key === 'ArrowRight') hue.value = (hue.value + step) % 360
	else if (event.key === 'ArrowUp') saturation.value = Math.min(1, saturation.value + step / 100)
	else if (event.key === 'ArrowDown') saturation.value = Math.max(0, saturation.value - step / 100)
	else return
	event.preventDefault()
	emit()
}

function onBrightness(percent: number): void {
	value.value = percent / 100
	emit()
}

function commitHex(): void {
	const color = `#${hexInput.value.replace('#', '').toLowerCase()}`
	if (hexToHsv(color)) model.value = color
	else hexInput.value = model.value.replace('#', '').toUpperCase()
}

const thumbStyle = computed(() => {
	const angle = (hue.value * Math.PI) / 180
	return {
		left: `${50 + Math.sin(angle) * saturation.value * 50}%`,
		top: `${50 - Math.cos(angle) * saturation.value * 50}%`,
		background: model.value,
	}
})
</script>

<template>
	<div
		class="flex flex-wrap items-center gap-6 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-4"
	>
		<div
			ref="wheel"
			class="color-wheel relative size-44 shrink-0 cursor-crosshair touch-none select-none rounded-full shadow-[inset_0_0_0_1px_rgb(0_0_0_/_20%)] outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
			:style="{ '--wheel-dim': 1 - value }"
			role="group"
			:aria-label="formatMessage(messages.wheel)"
			tabindex="0"
			@pointerdown="onPointerDown"
			@pointermove="onPointerMove"
			@keydown="onKeydown"
		>
			<span
				class="pointer-events-none absolute z-[1] size-[1.125rem] -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-solid border-white shadow-[0_0_0_1px_rgb(0_0_0_/_40%)]"
				:style="thumbStyle"
			/>
		</div>

		<div class="flex min-w-56 flex-1 flex-col gap-4">
			<div class="flex items-center gap-3">
				<span
					class="size-12 shrink-0 rounded-xl border border-solid border-surface-5"
					:style="{ background: model }"
					aria-hidden="true"
				/>
				<InputFrame appearance="surface" size="standard" class="w-full">
					<template #leading>
						<span class="text-secondary" aria-hidden="true">#</span>
					</template>
					<input
						v-model="hexInput"
						class="w-full min-w-0 appearance-none border-0 bg-transparent p-0 text-base font-medium uppercase shadow-none outline-none focus:ring-0"
						:aria-label="formatMessage(messages.hex)"
						maxlength="7"
						spellcheck="false"
						@keydown.enter="commitHex"
						@blur="commitHex"
					/>
				</InputFrame>
			</div>
			<div class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.brightness) }}</span>
				<Slider
					:model-value="Math.round(value * 100)"
					:aria-label="formatMessage(messages.brightness)"
					:min="0"
					:max="100"
					:step="1"
					unit="%"
					@update:model-value="onBrightness"
				/>
			</div>
		</div>
	</div>
</template>

<style scoped>
.color-wheel {
	background:
		radial-gradient(closest-side, white, rgb(255 255 255 / 0)),
		conic-gradient(red, yellow, lime, aqua, blue, magenta, red);
}

/* Brightness, laid over the whole wheel so it shows the colours on offer. */
.color-wheel::before {
	content: '';
	position: absolute;
	inset: 0;
	border-radius: inherit;
	background: black;
	opacity: var(--wheel-dim);
	pointer-events: none;
}
</style>
