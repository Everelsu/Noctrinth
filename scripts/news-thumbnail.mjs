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

	/**
	 * CurseForge back in the launcher: Discover with the source switch on
	 * CurseForge, its cards in CurseForge's orange, and a returning arrow.
	 */
	'curseforge-is-back': () => {
		const orange = '#f16436'
		const parts = [
			`<rect x="190" y="110" width="1540" height="1100" rx="48" fill="#17111f" fill-opacity="0.62" stroke="#fff" stroke-opacity="0.14" stroke-width="3"/>`,
			// The launcher's sidebar.
			`<rect x="190" y="110" width="120" height="1100" rx="48" fill="#fff" fill-opacity="0.04"/>`,
			`<rect x="222" y="150" width="56" height="56" rx="16" fill="url(#brand)"/>`,
			...[0, 1, 2, 3].map(
				(k) =>
					`<rect x="230" y="${260 + k * 90}" width="40" height="40" rx="12" fill="#fff" fill-opacity="${k === 0 ? 0.3 : 0.12}"/>`,
			),
			// Search and the source switch, set to CurseForge.
			`<rect x="360" y="150" width="760" height="64" rx="32" fill="#fff" fill-opacity="0.07" stroke="#fff" stroke-opacity="0.12" stroke-width="3"/>`,
			`<circle cx="400" cy="182" r="13" fill="none" stroke="#fff" stroke-opacity="0.35" stroke-width="5"/>`,
			bar(432, 170, 300, 24, '#fff', 0.16),
			`<rect x="1160" y="150" width="520" height="64" rx="32" fill="#fff" fill-opacity="0.06" stroke="#fff" stroke-opacity="0.12" stroke-width="3"/>`,
			`<circle cx="1206" cy="182" r="13" fill="#3bd16f" fill-opacity="0.75"/>`,
			bar(1232, 170, 150, 24, '#fff', 0.28),
			`<rect x="1424" y="156" width="250" height="52" rx="26" fill="${orange}"/>`,
			`<circle cx="1462" cy="182" r="13" fill="#2a0e05" fill-opacity="0.55"/>`,
			bar(1488, 170, 160, 24, '#2a0e05', 0.55),
		]
		// Project cards, two rows of three, with CurseForge's orange icons.
		const tints = ['#f16436', '#ff8a4c', '#e2502a', '#ff9f66', '#f47a3c', '#d9441f']
		tints.forEach((tint, k) => {
			const x = 360 + (k % 3) * 445
			const y = 270 + Math.floor(k / 3) * 300
			parts.push(
				panel(x, y, 420, 270, 28),
				`<rect x="${x + 28}" y="${y + 28}" width="96" height="96" rx="22" fill="${tint}" fill-opacity="0.85"/>`,
				bar(x + 148, y + 40, [190, 220, 160, 200, 230, 180][k], 28, '#fff', 0.42),
				bar(x + 148, y + 86, [130, 110, 150, 120, 100, 140][k], 20, '#fff', 0.2),
				bar(x + 28, y + 156, 360, 20, '#fff', 0.16),
				bar(x + 28, y + 190, [280, 300, 250, 320, 260, 290][k], 20, '#fff', 0.12),
				`<rect x="${x + 276}" y="${y + 210}" width="116" height="40" rx="20" fill="#c78aff" fill-opacity="${k === 1 ? 0.95 : 0.5}"/>`,
			)
		})
		// The returning arrow: an orange ring, nearly closed, with its head.
		const cx = 1560
		const cy = 900
		const r = 150
		const start = (-60 * Math.PI) / 180
		const end = (225 * Math.PI) / 180
		const sx = cx + r * Math.cos(start)
		const sy = cy + r * Math.sin(start)
		const ex = cx + r * Math.cos(end)
		const ey = cy + r * Math.sin(end)
		parts.push(
			`<circle cx="${cx}" cy="${cy}" r="${r + 70}" fill="#17111f" fill-opacity="0.75" stroke="#fff" stroke-opacity="0.14" stroke-width="3"/>`,
			`<path d="M ${sx.toFixed(1)} ${sy.toFixed(1)} A ${r} ${r} 0 1 1 ${ex.toFixed(1)} ${ey.toFixed(1)}" fill="none" stroke="url(#ember)" stroke-width="44" stroke-linecap="round"/>`,
			`<polygon points="${(sx + 6).toFixed(1)},${(sy - 70).toFixed(1)} ${(sx + 78).toFixed(1)},${(sy + 22).toFixed(1)} ${(sx - 34).toFixed(1)},${(sy + 34).toFixed(1)}" fill="${orange}"/>`,
		)
		return parts.join('')
	},
}

function thumbnail(scene, seed) {
	return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
<defs>
<linearGradient id="shade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#140c22" stop-opacity="0"/><stop offset="1" stop-color="#0d0817" stop-opacity="0.75"/></linearGradient>
<radialGradient id="glow" cx="0.3" cy="0.15" r="0.9"><stop offset="0" stop-color="#7d45d9" stop-opacity="0.22"/><stop offset="1" stop-color="#7d45d9" stop-opacity="0"/></radialGradient>
<linearGradient id="brand" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#e0b3ff"/><stop offset="1" stop-color="#9a5cf0"/></linearGradient>
<linearGradient id="ember" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff9f66"/><stop offset="1" stop-color="#e2502a"/></linearGradient>
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
