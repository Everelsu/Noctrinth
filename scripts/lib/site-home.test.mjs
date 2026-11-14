import assert from 'node:assert/strict'
import { setImmediate } from 'node:timers/promises'
import test from 'node:test'
import vm from 'node:vm'

import { renderHome } from './site-home.mjs'

const releasePage = 'https://github.com/Everelsu/Noctrinth/releases/latest'
const files = [
	'Noctrinth_0.23.3_arm64-setup.exe',
	'Noctrinth_0.23.3_x64-setup.exe',
	'Noctrinth_0.23.3_universal.dmg',
	'Noctrinth_0.23.3_aarch64.AppImage',
	'Noctrinth_0.23.3_amd64.AppImage',
	'Noctrinth_0.23.3_arm64.deb',
	'Noctrinth_0.23.3_amd64.deb',
]

async function browser({ ua, entropy, assets = files, fail = false, slowEntropy = false }) {
	const html = renderHome({ version: '0.23.99', articles: [] })
	const script = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].at(-1)[1]
	const links = Object.fromEntries(
		['windows', 'windowsArm', 'macos', 'appimage', 'appimageArm', 'deb', 'debArm'].map((key) => [
			key,
			{ href: releasePage, hidden: key.endsWith('Arm') },
		]),
	)
	const main = { href: releasePage }
	const label = {}
	const context = {
		navigator: {
			userAgent: ua,
			...(entropy
				? {
						userAgentData: {
							getHighEntropyValues: async () => {
								if (slowEntropy) await setImmediate()
								return entropy
							},
						},
					}
				: {}),
		},
		document: {
			getElementById: (id) => (id === 'download-main' ? main : label),
			querySelector: (selector) => links[/data-os="([^"]+)/.exec(selector)[1]],
		},
		fetch: async () => {
			if (fail) throw new Error('offline')
			return {
				ok: true,
				json: async () => ({
					tag_name: 'noctrinth-v0.23.3',
					html_url: releasePage,
					assets: assets.map((name) => ({
						name,
						browser_download_url: 'https://example.test/' + name,
					})),
				}),
			}
		},
	}
	vm.runInNewContext(script, context)
	await setImmediate()
	await setImmediate()
	return { html, main, label, links }
}

for (const [name, ua, entropy, expected] of [
	[
		'Windows x64',
		'Windows NT 10.0; Win64; x64',
		{ architecture: 'x86', bitness: '64' },
		'_x64-setup.exe',
	],
	[
		'Windows ARM with frozen x64 UA',
		'Windows NT 10.0; Win64; x64',
		{ architecture: 'arm', bitness: '64' },
		'_arm64-setup.exe',
	],
	['Linux x64', 'Linux x86_64', undefined, '_amd64.AppImage'],
	['Linux ARM', 'Linux aarch64', undefined, '_aarch64.AppImage'],
	['macOS Intel', 'Macintosh; Intel Mac OS X', undefined, '_universal.dmg'],
	['macOS ARM', 'Macintosh', { architecture: 'arm', bitness: '64' }, '_universal.dmg'],
]) {
	test(name, async () => {
		const { main, label } = await browser({ ua, entropy })
		assert.ok(main.href.endsWith(expected), main.href)
		assert.match(label.textContent, /v0\.23\.3$/)
	})
}

test('late architecture information updates the selected installer', async () => {
	const { main } = await browser({
		ua: 'Windows Win64 x64',
		entropy: { architecture: 'arm', bitness: '64' },
		slowEntropy: true,
	})
	assert.ok(main.href.endsWith('_arm64-setup.exe'))
})

test('unavailable ARM installers stay hidden and are never selected', async () => {
	const { main, links } = await browser({
		ua: 'Linux aarch64',
		assets: files.filter((name) => !/arm64|aarch64/.test(name)),
	})
	assert.equal(main.href, releasePage)
	assert.equal(links.appimageArm.hidden, true)
	assert.equal(links.windowsArm.hidden, true)
	assert.equal(links.debArm.hidden, true)
})

test('unknown Windows architecture uses the release page and explicit choices', async () => {
	const { main, links } = await browser({ ua: 'Windows NT 10.0; Win64; x64' })
	assert.equal(main.href, releasePage)
	assert.ok(links.windows.href.endsWith('_x64-setup.exe'))
	assert.ok(links.windowsArm.href.endsWith('_arm64-setup.exe'))
})

test('API failure and no JavaScript never link to an unreleased build', async () => {
	const { html, main, label } = await browser({ ua: 'Linux x86_64', fail: true })
	assert.equal(main.href, releasePage)
	assert.doesNotMatch(html, /releases\/latest\/download\//)
	assert.doesNotMatch(label.textContent, /0\.23\.99/)
})
