<template>
	<!-- eslint-disable vue/no-undef-components -->
	<div
		ref="skinPreviewContainer"
		class="relative w-full h-full overflow-visible cursor-grab"
		@click="onCanvasClick"
		@wheel="onWheel"
	>
		<div
			data-skin-preview-debug="controls"
			class="absolute left-0 right-0 z-10 flex items-center justify-center pointer-events-none"
			:style="previewControlsPositionStyle"
		>
			<span
				class="flex items-center justify-center gap-1.5 text-base font-medium leading-6 text-primary"
			>
				<UnfoldHorizontalIcon class="size-5 shrink-0" />
				{{ formatMessage(messages.dragToRotate) }}
			</span>
		</div>
		<div
			v-if="$slots.subtitle"
			data-skin-preview-debug="subtitle"
			class="absolute left-0 right-0 z-10 flex items-center justify-center pointer-events-none"
			:style="subtitlePositionStyle"
		>
			<div ref="subtitleElement" class="w-full pointer-events-auto" @click="ignoreControlClick">
				<slot name="subtitle" />
			</div>
		</div>
		<div
			v-if="nametag || $slots['nametag-badge']"
			data-skin-preview-debug="nametag"
			class="absolute left-1/2 pointer-events-none z-10"
			:style="nametagStyle"
		>
			<div
				v-if="$slots['nametag-badge']"
				class="absolute bottom-[calc(100%+1rem)] left-1/2 flex -translate-x-1/2 items-center justify-center"
			>
				<slot name="nametag-badge" />
			</div>
			<div v-if="nametag" class="px-3 py-1 rounded-md font-minecraft text-gray nametag-bg">
				{{ nametagText }}
			</div>
		</div>

		<TresCanvas
			alpha
			:antialias="true"
			:dpr="rendererDpr"
			:renderer-options="{
				outputColorSpace: THREE.SRGBColorSpace,
				toneMapping: THREE.NoToneMapping,
				toneMappingExposure: 10.0,
			}"
			class="transition-opacity duration-500"
			:class="{ 'opacity-0': !isPreviewVisible, 'opacity-100': isPreviewVisible }"
			@pointerdown="onPointerDown"
			@pointermove="onPointerMove"
			@pointerup="onPointerUp"
			@pointerleave="onPointerUp"
		>
			<Suspense>
				<Group
					:rotation="animatedModelGroupRotation"
					:position="animatedModelGroupPosition"
					:scale="animatedModelGroupScale"
				>
					<Group :position="modelOffset">
						<primitive v-if="scene" :object="scene" />
						<!--
							Shadow is a child of the same group as the model —
							it inherits position, rotation, and scale, so it
							stays glued to the feet through every drag. The
							[-π/2, 0, 0] rotation lays it flat IN THE MODEL'S
							LOCAL FRAME, so when the figure pitches, the
							shadow plane pitches with it.
						-->
						<TresMesh
							:position="shadowLocalPosition"
							:rotation="[-Math.PI / 2, 0, 0]"
							:scale="spotlightScale"
						>
							<TresCircleGeometry :args="[1, 128]" />
							<TresShaderMaterial v-bind="radialSpotlightShader" />
						</TresMesh>
					</Group>
				</Group>
			</Suspense>

			<TresPerspectiveCamera
				:make-default.camel="true"
				:fov="cameraConfig.fov"
				:position="cameraConfig.position"
				:look-at="cameraConfig.target"
			/>

			<TresAmbientLight :intensity="2" />
			<TresDirectionalLight :position="[-3, 4, -2]" :intensity="1.2" />
		</TresCanvas>

		<div v-if="showLoading" class="absolute inset-0 flex items-center justify-center">
			<div class="text-primary">Loading...</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ClassicPlayerModel, SlimPlayerModel, UnfoldHorizontalIcon } from '@modrinth/assets'
import { TresCanvas } from '@tresjs/core'
import * as THREE from 'three'
import {
	computed,
	nextTick,
	onMounted,
	onUnmounted,
	ref,
	toRef,
	useSlots,
	useTemplateRef,
	watch,
} from 'vue'

import type {
	SkinPreviewAnimationConfig,
	SkinPreviewFitPadding,
	SkinPreviewFraming,
	SkinPreviewTuple,
} from '#ui/composables/skin-rendering'
import {
	useSkinPreviewAnimation,
	useSkinPreviewControls,
	useSkinPreviewFit,
	useSkinPreviewLoading,
	useSkinPreviewScene,
} from '#ui/composables/skin-rendering'

import { useDynamicFontSize } from '../../composables'
import { defineMessages, useVIntl } from '../../composables/i18n'
import { createRadialSpotlightShader, syncDamageFlashShader } from './skin-preview-shader'

const props = withDefaults(
	defineProps<{
		textureSrc: string
		earsTextureSrc?: string
		capeSrc?: string
		variant?: 'SLIM' | 'CLASSIC' | 'UNKNOWN'
		nametag?: string
		fit?: boolean
		lockFit?: boolean
		framing?: SkinPreviewFraming
		fitZoom?: number
		fitPadding?: Partial<SkinPreviewFitPadding>
		/** @deprecated Manual framing fallback. */
		scale?: number
		/** @deprecated Manual framing fallback, or auto-fit FOV override when fit=true. */
		fov?: number
		initialRotation?: number
		animationConfig?: SkinPreviewAnimationConfig
		earsEnabled?: boolean
	}>(),
	{
		variant: 'CLASSIC',
		earsTextureSrc: undefined,
		capeSrc: undefined,
		initialRotation: 15.75,
		nametag: undefined,
		fit: undefined,
		lockFit: true,
		framing: 'page',
		fitZoom: 1,
		earsEnabled: true,
		animationConfig: () => ({
			baseAnimation: 'idle',
			randomAnimations: ['idle_sub_1', 'idle_sub_2', 'idle_sub_3'],
			randomAnimationInterval: 8000,
			transitionDuration: 0.2,
		}),
	},
)

const { formatMessage } = useVIntl()

const messages = defineMessages({
	dragToRotate: {
		id: 'skin.preview.drag-to-rotate',
		defaultMessage: 'Drag to rotate',
	},
})

const emit = defineEmits<{
	earsFeaturesDetected: [detected: boolean]
}>()

const skinPreviewContainer = useTemplateRef<HTMLElement>('skinPreviewContainer')
const subtitleElement = useTemplateRef<HTMLElement>('subtitleElement')
const slots = useSlots()
const nametagText = computed(() => props.nametag)
const hasSubtitle = computed(() => Boolean(slots.subtitle))
const hasNametagBadge = computed(() => Boolean(slots['nametag-badge']))
const isSubtitleWrapped = ref(false)
const selectedModelSrc = computed(() =>
	props.variant === 'SLIM' ? SlimPlayerModel : ClassicPlayerModel,
)

let subtitleResizeObserver: ResizeObserver | undefined

function getSubtitleLayoutRoot(element: HTMLElement) {
	const elementChildren = Array.from(element.children).filter(
		(child): child is HTMLElement => child instanceof HTMLElement,
	)

	return elementChildren.length === 1 ? elementChildren[0] : element
}

function updateSubtitleWrapped() {
	const element = subtitleElement.value
	if (!element) {
		isSubtitleWrapped.value = false
		return
	}

	const layoutRoot = getSubtitleLayoutRoot(element)
	const children = Array.from(layoutRoot.children).filter(
		(child): child is HTMLElement => child instanceof HTMLElement,
	)

	if (children.length < 2) {
		isSubtitleWrapped.value = false
		return
	}

	const firstTop = children[0].getBoundingClientRect().top
	isSubtitleWrapped.value = children.some(
		(child) => Math.abs(child.getBoundingClientRect().top - firstTop) > 1,
	)
}

function observeSubtitleElement() {
	subtitleResizeObserver?.disconnect()

	const element = subtitleElement.value
	if (!element) {
		isSubtitleWrapped.value = false
		return
	}

	const layoutRoot = getSubtitleLayoutRoot(element)

	subtitleResizeObserver = new ResizeObserver(updateSubtitleWrapped)
	subtitleResizeObserver.observe(element)
	if (layoutRoot !== element) {
		subtitleResizeObserver.observe(layoutRoot)
	}

	void nextTick(updateSubtitleWrapped)
}

const {
	cleanupAnimationState,
	clickImpulseOffsetX,
	clickImpulseRotationZ,
	clickImpulseScaleX,
	clickImpulseScaleY,
	currentAnimation,
	damageFlashIntensity,
	getAvailableAnimations,
	initializeAnimations,
	playAnimation,
	playClickInteraction,
	stopAnimations,
} = useSkinPreviewAnimation(toRef(props, 'animationConfig'))

const {
	ignoreControlClick,
	modelPitch,
	modelRotation,
	modelZoom,
	onCanvasClick,
	onPointerDown,
	onPointerMove,
	onPointerUp,
	onWheel,
} = useSkinPreviewControls({
	initialRotation: toRef(props, 'initialRotation'),
	onClickWithoutDrag: () => {
		playClickInteraction()
	},
})

const {
	hasEarsFeatures,
	isModelLoaded,
	isTextureLoaded,
	modelCenter,
	modelSize,
	scene,
	visibleBounds,
} = useSkinPreviewScene({
	selectedModelSrc,
	textureSrc: toRef(props, 'textureSrc'),
	earsTextureSrc: toRef(props, 'earsTextureSrc'),
	capeSrc: toRef(props, 'capeSrc'),
	earsEnabled: toRef(props, 'earsEnabled'),
	initializeAnimations,
	cleanupAnimationState,
})

function syncDamageFlashShaderMaterials() {
	syncDamageFlashShader(scene.value, damageFlashIntensity.value)
}

const {
	cameraConfig,
	fitEnabled,
	hasResolvedFit,
	modelGroupPosition,
	modelGroupScale,
	modelOffset,
	nametagTop,
	previewControlsPositionStyle,
	spotlightScale,
	subtitlePositionStyle,
} = useSkinPreviewFit({
	containerElement: computed(() => skinPreviewContainer.value),
	fit: toRef(props, 'fit'),
	lockFit: toRef(props, 'lockFit'),
	framing: toRef(props, 'framing'),
	fitZoom: toRef(props, 'fitZoom'),
	fitPadding: toRef(props, 'fitPadding'),
	scale: toRef(props, 'scale'),
	fov: toRef(props, 'fov'),
	modelRotation,
	nametag: toRef(props, 'nametag'),
	hasSubtitle,
	hasNametagBadge,
	subtitleWrapped: isSubtitleWrapped,
	modelCenter,
	modelSize,
	scene,
	visibleBounds,
	isModelLoaded,
})

const rendererDpr: [number, number] = [1, 1.5]
const radialSpotlightShader = createRadialSpotlightShader()
const isReady = computed(() => isModelLoaded.value && isTextureLoaded.value && hasResolvedFit.value)
const { isPreviewVisible, showLoading } = useSkinPreviewLoading(isReady)

onMounted(observeSubtitleElement)

watch(hasSubtitle, () => nextTick(observeSubtitleElement), { flush: 'post' })
watch(
	hasEarsFeatures,
	(detected) => {
		emit('earsFeaturesDetected', detected)
	},
	{ immediate: true },
)
watch(scene, syncDamageFlashShaderMaterials, { immediate: true })
watch(damageFlashIntensity, syncDamageFlashShaderMaterials)

onUnmounted(() => {
	subtitleResizeObserver?.disconnect()
})

const { fontSize: nametagFontSize } = useDynamicFontSize({
	containerElement: skinPreviewContainer,
	text: nametagText,
	baseFontSize: 1.8,
	minFontSize: 1.25,
	maxFontSize: 2,
	padding: 24,
	fontFamily: 'inherit',
})

const nametagStyle = computed(() => ({
	fontSize: nametagFontSize.value,
	top: nametagTop.value,
	transform: fitEnabled.value ? 'translate(-50%, calc(-100% - 0.75rem))' : 'translateX(-50%)',
}))

const animatedModelGroupRotation = computed<SkinPreviewTuple>(() => [
	modelPitch.value,
	modelRotation.value,
	clickImpulseRotationZ.value,
])

const animatedModelGroupPosition = computed<SkinPreviewTuple>(() => {
	const [x, y, z] = modelGroupPosition.value
	return [x + clickImpulseOffsetX.value, y, z]
})

const animatedModelGroupScale = computed<SkinPreviewTuple>(() => {
	const [x, y, z] = modelGroupScale.value
	const zoom = modelZoom.value
	return [x * clickImpulseScaleX.value * zoom, y * clickImpulseScaleY.value * zoom, z * zoom]
})

// Shadow lives INSIDE the model group, so it inherits position+rotation+scale
// from the same transform stack as the model itself — it's truly attached to
// the figure's feet. When the user spins or tilts the preview, the shadow
// follows.
//
// Local Y inside the inner group: matches what the legacy world-space
// spotlight used (`-sizeY/2 - epsilon`) but expressed in the inner frame.
// The inner group is offset by `modelOffset = -modelCenter`, so a world Y of
// `-sizeY/2 - eps` translates to local Y of `modelCenter.y - sizeY/2 - eps`.
// (For a model whose local origin is at the feet — Mojang's player.gltf —
// this lands ~`-eps` just below the soles regardless of model height.)
const SHADOW_FEET_EPSILON = 0.02
const shadowLocalPosition = computed<SkinPreviewTuple>(() => {
	const [, cy] = modelCenter.value
	const [, sizeY] = modelSize.value
	return [0, cy - sizeY / 2 - SHADOW_FEET_EPSILON, 0]
})

defineExpose({
	playAnimation,
	playClickInteraction,
	stopAnimations,
	getAvailableAnimations,
	getCurrentAnimation: () => currentAnimation.value,
})
</script>

<style scoped lang="scss">
.nametag-bg {
	background:
		linear-gradient(308.68deg, rgba(50, 50, 50, 0.2) -52.46%, rgba(100, 100, 100, 0.2) 94.75%),
		rgba(0, 0, 0, 0.2);
	box-shadow:
		inset -0.5px -0.5px 0px rgba(0, 0, 0, 0.25),
		inset 0.5px 0.5px 0px rgba(255, 255, 255, 0.05);
}
</style>
