<script setup lang="ts">
import { ref } from 'vue'

const emit = defineEmits<{
	(e: 'select'): void
	(e: 'edit', event: MouseEvent): void
}>()

const props = withDefaults(
	defineProps<{
		forwardImageSrc?: string
		backwardImageSrc?: string
		selected: boolean
		tooltip?: string
	}>(),
	{
		forwardImageSrc: undefined,
		backwardImageSrc: undefined,
		tooltip: undefined,
	},
)

const imagesLoaded = ref({
	forward: Boolean(props.forwardImageSrc),
	backward: Boolean(props.backwardImageSrc),
})

function onImageLoad(type: 'forward' | 'backward') {
	imagesLoaded.value[type] = true
}
</script>

<template>
	<!--
		Selected ring uses `outline` (not `border`) with positive offset so the
		highlight sits OUTSIDE the container, not inside an `overflow-hidden`
		clip where the rounded corners would chew off the skin tile's edges.
	-->
	<div
		v-tooltip="tooltip ?? undefined"
		class="group flex relative overflow-hidden rounded-xl transition-[outline-color] duration-200"
		:class="[
			selected
				? 'outline outline-[3px] outline-brand outline-offset-2'
				: 'outline outline-2 outline-transparent outline-offset-2 hover:outline-inverted',
		]"
	>
		<button
			class="skin-btn-bg absolute inset-0 cursor-pointer p-0 border-none group-hover:brightness-125"
			:class="selected ? 'selected' : ''"
			@click="emit('select')"
		></button>

		<div
			v-if="!(imagesLoaded.forward && imagesLoaded.backward)"
			class="skeleton-loader w-full h-full"
		>
			<div class="skeleton absolute inset-0 aspect-[5/7]"></div>
		</div>

		<span
			v-show="imagesLoaded.forward && imagesLoaded.backward"
			:class="[
				'skin-button__image-parent pointer-events-none w-full h-full grid [transform-style:preserve-3d] transition-transform duration-500 group-hover:[transform:rotateY(180deg)] place-items-stretch with-shadow',
			]"
		>
			<img
				alt=""
				:src="forwardImageSrc"
				class="skin-button__image-facing object-contain w-full h-full [backface-visibility:hidden] col-start-1 row-start-1"
				height="504"
				@load="onImageLoad('forward')"
			/>
			<img
				alt=""
				:src="backwardImageSrc"
				class="skin-button__image-away object-contain w-full h-full [backface-visibility:hidden] [transform:rotateY(180deg)] col-start-1 row-start-1"
				height="504"
				@load="onImageLoad('backward')"
			/>
		</span>

		<span
			v-if="$slots['overlay-buttons']"
			class="pointer-events-none absolute inset-0 flex items-end justify-start p-1 gap-1 translate-y-4 scale-75 opacity-0 transition-all group-hover:opacity-100 group-hover:scale-100 group-hover:translate-y-0 group-hover:translate-x-0"
		>
			<slot name="overlay-buttons" />
		</span>
	</div>
</template>

<style scoped lang="scss">
.skeleton-loader {
	aspect-ratio: 5 / 7;
}

.skeleton {
	background: linear-gradient(
		90deg,
		var(--color-bg) 25%,
		var(--color-raised-bg) 50%,
		var(--color-bg) 75%
	);
	background-size: 200% 100%;
	animation: wave 1500ms infinite linear;
}

@keyframes wave {
	0% {
		background-position: -200% 0;
	}
	100% {
		background-position: 200% 0;
	}
}

.skin-btn-bg {
	background: var(--color-gradient-button-bg);
}

.skin-btn-bg.selected {
	background:
		linear-gradient(
			157.61deg,
			var(--color-brand) -76.68%,
			rgba(199, 138, 255, 0.4) -38.61%,
			rgba(80, 30, 130, 0.6) 100.4%
		),
		var(--color-bg);
}

.skin-btn-bg.selected:hover,
.group:hover .skin-btn-bg.selected {
	filter: brightness(1.15);
}

.with-shadow img {
	filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.4));
}

.skin-button__image-parent img {
	transition: filter 200ms ease-in-out;
}

.group:hover .skin-button__image-parent img {
	filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
}

.with-shadow img {
	filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.4));
}
</style>
