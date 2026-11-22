import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { type MaybeRefOrGetter, ref, toValue, watch } from 'vue'

const thumbnailCache = new Map<string, string>()

export function useImageThumbnail(
	path: MaybeRefOrGetter<string | null | undefined>,
	size: MaybeRefOrGetter<number>,
	revision: MaybeRefOrGetter<string | number | undefined> = undefined,
) {
	const thumbnail = ref<string>()
	watch(
		[() => toValue(path), () => toValue(size), () => toValue(revision)],
		([source, pixels, rev], [previousSource, previousPixels], onCleanup) => {
			if (!source) {
				thumbnail.value = undefined
				return
			}
			const cacheKey = `${source}::${pixels}::${rev ?? ''}`
			const cached = thumbnailCache.get(cacheKey)
			if (cached) {
				thumbnail.value = cached
				return
			}
			if (source !== previousSource || pixels !== previousPixels) {
				thumbnail.value = undefined
			}
			let active = true
			onCleanup(() => {
				active = false
			})
			void invoke<string>('plugin:utils|get_image_thumbnail', {
				path: source,
				size: pixels,
			})
				.then((url) => {
					thumbnailCache.set(cacheKey, url)
					if (active) thumbnail.value = url
				})
				.catch((error) => {
					console.warn('Could not create image thumbnail', error)
					const fallback = convertFileSrc(source)
					thumbnailCache.set(cacheKey, fallback)
					if (active) thumbnail.value = fallback
				})
		},
		{ immediate: true },
	)
	return thumbnail
}
