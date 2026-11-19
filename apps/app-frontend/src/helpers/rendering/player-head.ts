export async function generatePlayerHeadBlob(skinUrl: string, size: number = 64): Promise<Blob> {
	return new Promise((resolve, reject) => {
		const img = new Image()
		img.crossOrigin = 'anonymous'

		img.onload = () => {
			try {
				const sourceCanvas = document.createElement('canvas')
				const sourceCtx = sourceCanvas.getContext('2d')

				if (!sourceCtx) {
					throw new Error('Could not get 2D context from source canvas')
				}

				sourceCanvas.width = img.width
				sourceCanvas.height = img.height

				sourceCtx.drawImage(img, 0, 0)

				const outputCanvas = document.createElement('canvas')
				const outputCtx = outputCanvas.getContext('2d')

				if (!outputCtx) {
					throw new Error('Could not get 2D context from output canvas')
				}

				outputCanvas.width = size
				outputCanvas.height = size

				outputCtx.imageSmoothingEnabled = false

				const headImageData = sourceCtx.getImageData(8, 8, 8, 8)

				const headCanvas = document.createElement('canvas')
				const headCtx = headCanvas.getContext('2d')

				if (!headCtx) {
					throw new Error('Could not get 2D context from head canvas')
				}

				headCanvas.width = 8
				headCanvas.height = 8
				headCtx.putImageData(headImageData, 0, 0)

				outputCtx.drawImage(headCanvas, 0, 0, 8, 8, 0, 0, size, size)

				const hatImageData = sourceCtx.getImageData(40, 8, 8, 8)

				const hatCanvas = document.createElement('canvas')
				const hatCtx = hatCanvas.getContext('2d')

				if (!hatCtx) {
					throw new Error('Could not get 2D context from hat canvas')
				}

				hatCanvas.width = 8
				hatCanvas.height = 8
				hatCtx.putImageData(hatImageData, 0, 0)

				const hatPixels = hatImageData.data
				let hasHat = false

				for (let i = 3; i < hatPixels.length; i += 4) {
					if (hatPixels[i] > 0) {
						hasHat = true
						break
					}
				}

				if (hasHat) {
					outputCtx.drawImage(hatCanvas, 0, 0, 8, 8, 0, 0, size, size)
				}

				outputCanvas.toBlob(
					(blob) => {
						if (blob) {
							resolve(blob)
						} else {
							reject(new Error('Failed to create blob from canvas'))
						}
					},
					'image/webp',
					0.9,
				)
			} catch (error) {
				reject(error)
			}
		}

		img.onerror = () => {
			reject(new Error('Failed to load skin texture image'))
		}

		img.src = skinUrl
	})
}
