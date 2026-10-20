import {
	type ImageViewerEditorData,
	type ImageViewerEditorSource,
	provideImageViewerEditor,
} from '@modrinth/ui'
import { readFile } from '@tauri-apps/plugin-fs'

import { release_ads_window_hold, take_ads_window_hold } from '@/helpers/ads.js'
import { proxiedFetch } from '@/helpers/proxy-fetch'

/**
 * Noctrinth's own: CurseForge's CDN sends no CORS headers, so the page cannot
 * read its screenshots; the app's own HTTP client can.
 */
async function fetchImage(src: string): Promise<Blob> {
	const response = await fetch(src).catch(() => proxiedFetch(src))
	if (!response.ok) throw new Error(`Could not load image: ${response.statusText}`)
	return await response.blob()
}

export function setupImageViewerEditorProvider() {
	provideImageViewerEditor({
		async loadEditorData(source: ImageViewerEditorSource): Promise<ImageViewerEditorData> {
			return {
				source: new Blob([await readFile(source.path)], { type: 'image/png' }),
			}
		},
		fetchImage,
		onShow: take_ads_window_hold,
		onHide: release_ads_window_hold,
	})
}
