#!/usr/bin/env node
// Draws a news thumbnail in the style of Modrinth's blog, in Noctrinth's
// purple: isometric blocks in the back, a simplified picture of the thing the
// article is about in front, all as plain shapes.
//
// Usage:
//   node scripts/news-thumbnail.mjs <scene> <output.svg> [seed]
//
// <scene> is one of the scenes below. A new article gets a scene of its own:
// copy one, change what is drawn in front, and keep the background as it is.
// The seed only moves the blocks around.

import { writeFileSync } from 'node:fs'

const W = 1920
const H = 1080

/** A small seeded generator, so a thumbnail can be drawn again the same. */
function random(seed) {
	let state = seed >>> 0 || 1
	return () => {
		state = (state * 1664525 + 1013904223) >>> 0
		return state / 2 ** 32
	}
}

function blocks(seed) {
	const next = random(seed)
	const side = 230
	const dx = side * 0.866
	const dy = side * 0.5
	const cells = []
	for (let i = -4; i < 16; i++) {
		for (let j = -4; j < 16; j++) {
			const x = (i - j) * dx + W / 2
			const y = (i + j) * dy - 900
			if (x > -400 && x < W + 400 && y > -400 && y < H + 700) {
				const height = [0, 0, 1, 1, 2, 3][Math.floor(next() * 6)] * side * 0.6
				cells.push({ depth: i + j, i, x, y, height })
			}
		}
	}
	cells.sort((a, b) => a.depth - b.depth || a.i - b.i)

	const r = (n) => Math.round(n)
	return cells
		.map(({ x, y, height }) => {
			const t = y - height
			const top = `${r(x)},${r(t)} ${r(x + dx)},${r(t + dy)} ${r(x)},${r(t + 2 * dy)} ${r(x - dx)},${r(t + dy)}`
			const left = `${r(x - dx)},${r(t + dy)} ${r(x)},${r(t + 2 * dy)} ${r(x)},${r(y + 2 * dy + side)} ${r(x - dx)},${r(y + dy + side)}`
			const right = `${r(x + dx)},${r(t + dy)} ${r(x)},${r(t + 2 * dy)} ${r(x)},${r(y + 2 * dy + side)} ${r(x + dx)},${r(y + dy + side)}`
			return `<polygon points="${left}" fill="#21182f"/><polygon points="${right}" fill="#191226"/><polygon points="${top}" fill="#2f2347"/>`
		})
		.join('')
}

/** A rounded line standing in for text. */
const bar = (x, y, w, h = 28, fill = '#fff', opacity = 0.22) =>
	`<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${h / 2}" fill="${fill}" fill-opacity="${opacity}"/>`

/** A see-through card with a faint edge. */
const panel = (x, y, w, h, radius = 32, opacity = 0.07) =>
	`<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${radius}" fill="#fff" fill-opacity="${opacity}" stroke="#fff" stroke-opacity="0.14" stroke-width="3"/>`

const SCENES = {
	/** The site itself: a browser window with the front page in it. */
	'new-site': () => {
		const parts = [
			`<rect x="190" y="110" width="1540" height="1100" rx="48" fill="#17111f" fill-opacity="0.62" stroke="#fff" stroke-opacity="0.14" stroke-width="3"/>`,
			...[0, 1, 2].map(
				(k) =>
					`<circle cx="${240 + k * 48}" cy="160" r="17" fill="#fff" fill-opacity="0.16" stroke="#fff" stroke-opacity="0.12" stroke-width="2"/>`,
			),
			bar(560, 142, 800, 36, '#fff', 0.1),
			`<line x1="190" y1="210" x2="1730" y2="210" stroke="#fff" stroke-opacity="0.12" stroke-width="3"/>`,
			`<rect x="250" y="250" width="64" height="64" rx="18" fill="url(#brand)"/>`,
			bar(336, 268, 170, 28, '#fff', 0.3),
			bar(1180, 268, 110, 28, '#fff', 0.18),
			bar(1320, 268, 150, 28, '#fff', 0.18),
			bar(1500, 268, 90, 28, '#fff', 0.18),
			`<rect x="250" y="360" width="1420" height="330" rx="36" fill="url(#hero)" stroke="#d6a8ff" stroke-opacity="0.35" stroke-width="3"/>`,
			bar(320, 430, 620, 52, '#f6edff', 0.85),
			bar(320, 510, 460, 30, '#f6edff', 0.4),
			bar(320, 560, 380, 30, '#f6edff', 0.4),
			`<rect x="320" y="612" width="300" height="56" rx="28" fill="#c78aff"/>`,
			`<circle cx="356" cy="640" r="14" fill="#1b1027" fill-opacity="0.75"/>`,
			bar(384, 628, 200, 24, '#1b1027', 0.6),
			`<rect x="1080" y="400" width="530" height="250" rx="28" fill="#fff" fill-opacity="0.1" stroke="#fff" stroke-opacity="0.18" stroke-width="3"/>`,
			`<rect x="1110" y="430" width="470" height="120" rx="18" fill="#9a5cf0" fill-opacity="0.55"/>`,
			bar(1110, 572, 320, 24, '#fff', 0.45),
			bar(1110, 610, 220, 20, '#fff', 0.22),
		]
		;['#b07cff', '#8f6bff', '#d08cff'].forEach((tint, k) => {
			const x = 250 + k * 485
			parts.push(
				panel(x, 730, 450, 420),
				`<rect x="${x + 24}" y="754" width="402" height="190" rx="20" fill="${tint}" fill-opacity="0.55"/>`,
				bar(x + 24, 970, [300, 250, 330][k], 26, '#fff', 0.32),
				bar(x + 24, 1012, [220, 280, 180][k], 20, '#fff', 0.16),
			)
		})
		return parts.join('')
	},
}

function thumbnail(scene, seed) {
	return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
<defs>
<linearGradient id="shade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#140c22" stop-opacity="0"/><stop offset="1" stop-color="#0d0817" stop-opacity="0.75"/></linearGradient>
<radialGradient id="glow" cx="0.3" cy="0.15" r="0.9"><stop offset="0" stop-color="#7d45d9" stop-opacity="0.22"/><stop offset="1" stop-color="#7d45d9" stop-opacity="0"/></radialGradient>
<linearGradient id="brand" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#e0b3ff"/><stop offset="1" stop-color="#9a5cf0"/></linearGradient>
<linearGradient id="hero" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#7a3fd1" stop-opacity="0.55"/><stop offset="1" stop-color="#3b1f6b" stop-opacity="0.35"/></linearGradient>
</defs>
<rect width="${W}" height="${H}" fill="#161022"/>
${blocks(seed)}
<rect width="${W}" height="${H}" fill="url(#glow)"/>
<rect width="${W}" height="${H}" fill="url(#shade)"/>
${scene()}
</svg>
`
}

const [, , sceneName, output, seedArg] = process.argv
const scene = SCENES[sceneName]
if (!scene || !output) {
	console.error(
		`Usage: news-thumbnail.mjs <scene> <output.svg> [seed]\nScenes: ${Object.keys(SCENES).join(', ')}`,
	)
	process.exit(1)
}

const svg = thumbnail(scene, Number(seedArg ?? 7))
writeFileSync(output, svg)
console.log(`Wrote ${sceneName} (${Math.round(svg.length / 1024)} KB) → ${output}`)
