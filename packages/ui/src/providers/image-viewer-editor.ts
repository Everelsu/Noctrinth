import type {
	ImageViewerEditorData,
	ImageViewerEditorSource,
} from '#ui/components/image-viewer-editor/types'

import { createContext } from '.'

export interface ImageViewerEditorContext {
	loadEditorData: (source: ImageViewerEditorSource) => Promise<ImageViewerEditorData>
	/** Loads an image by URL, for hosts the page itself may not fetch from. */
	fetchImage?: (src: string) => Promise<Blob>
	onShow?: () => void
	onHide?: () => void
}

export const [injectImageViewerEditor, provideImageViewerEditor] =
	createContext<ImageViewerEditorContext>('ImageViewerEditor')
