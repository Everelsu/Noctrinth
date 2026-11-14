import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import vm from 'node:vm'
import { parse } from 'yaml'

const workflow = parse(
	readFileSync(new URL('../.github/workflows/noctrinth-release.yml', import.meta.url), 'utf8'),
)
const job = workflow.jobs['publish-updater']
const script = job.steps.find((step) => step.run).run
const validate = /--slurpfile release release.json '([\s\S]*?)' manifest\/latest.json/.exec(
	script,
)[1]
const publicRelease = /jq -e '([^']+)' release\.json/.exec(script)[1]
const keys = [
	'windows-x86_64',
	'windows-aarch64',
	'linux-x86_64',
	'linux-aarch64',
	'darwin-x86_64',
	'darwin-aarch64',
	'windows-x86_64-nsis',
	'windows-aarch64-nsis',
	'linux-x86_64-appimage',
	'linux-aarch64-appimage',
	'linux-x86_64-deb',
	'linux-aarch64-deb',
	'darwin-x86_64-app',
	'darwin-aarch64-app',
]

function checkManifest(change = () => {}) {
	const manifest = {
		version: '0.23.3',
		platforms: Object.fromEntries(
			keys.map((key) => [key, { signature: 'signed-' + key, url: 'https://example.test/' + key }]),
		),
	}
	const release = {
		isDraft: false,
		isPrerelease: false,
		assets: keys.map((key) => ({ url: 'https://example.test/' + key })),
	}
	change(manifest, release)
	const directory = mkdtempSync(join(tmpdir(), 'noctrinth-manifest-'))
	try {
		const file = join(directory, 'release.json')
		writeFileSync(file, JSON.stringify(release))
		const result = spawnSync(
			process.env.JQ || 'jq',
			['-e', '--arg', 'version', '0.23.3', '--slurpfile', 'release', file, validate],
			{ input: JSON.stringify(manifest), encoding: 'utf8' },
		)
		assert.ifError(result.error)
		return result.status
	} finally {
		rmSync(directory, { recursive: true, force: true })
	}
}

test('updater publication runs only after stable publication or a successful micropatch', () => {
	assert.deepEqual(workflow.on.release.types, ['published'])
	assert.equal(workflow.jobs.release.if, "github.event_name != 'release'")
	for (const [event, prerelease, micropatch, result, expected] of [
		['push', false, undefined, 'success', false],
		['workflow_dispatch', false, 'false', 'success', false],
		['workflow_dispatch', false, 'true', 'success', true],
		['workflow_dispatch', false, 'true', 'failure', false],
		['release', false, undefined, 'skipped', true],
		['release', true, undefined, 'skipped', false],
	]) {
		assert.equal(
			vm.runInNewContext(job.if, {
				always: () => true,
				cancelled: () => false,
				github: { event_name: event, event: { release: { prerelease }, inputs: { micropatch } } },
				needs: { finalize: { result } },
			}),
			expected,
		)
	}
})

test('drafts and prereleases cannot publish an updater', () => {
	for (const [isDraft, isPrerelease, expected] of [
		[false, false, 0],
		[true, false, 1],
		[false, true, 1],
	]) {
		const result = spawnSync(process.env.JQ || 'jq', ['-e', publicRelease], {
			input: JSON.stringify({ isDraft, isPrerelease }),
		})
		assert.ifError(result.error)
		assert.equal(result.status, expected)
	}
})

test('complete signed manifest passes', () => assert.equal(checkManifest(), 0))
test('missing ARM platform fails', () =>
	assert.notEqual(
		checkManifest((manifest) => {
			delete manifest.platforms['windows-aarch64']
		}),
		0,
	))
test('empty signature fails', () =>
	assert.notEqual(
		checkManifest((manifest) => {
			manifest.platforms['linux-aarch64'].signature = ''
		}),
		0,
	))
test('missing installer fails', () =>
	assert.notEqual(
		checkManifest((manifest, release) => {
			release.assets.pop()
		}),
		0,
	))
test('wrong release version fails', () =>
	assert.notEqual(
		checkManifest((manifest) => {
			manifest.version = '0.23.1'
		}),
		0,
	))

test('missing deb updater target fails', () =>
	assert.notEqual(
		checkManifest((manifest) => {
			delete manifest.platforms['linux-aarch64-deb']
		}),
		0,
	))

test('release download table lists available installers without architecture images', () => {
	const directory = mkdtempSync(join(tmpdir(), 'noctrinth-notes-'))
	try {
		const file = join(directory, 'notes.md')
		const result = spawnSync(process.execPath, ['scripts/extract-changelog.mjs', '0.23.3', file], {
			env: {
				...process.env,
				RELEASE_ASSETS: JSON.stringify([
					'Noctrinth_0.23.3_x64-setup.exe',
					'Noctrinth_0.23.3_arm64.deb',
				]),
			},
			encoding: 'utf8',
		})
		assert.equal(result.status, 0, result.stderr)
		const body = readFileSync(file, 'utf8')
		assert.match(body, /Windows \| x64/)
		assert.match(body, /Linux \| ARM64/)
		assert.doesNotMatch(body, /download-windows\.svg/)
		assert.doesNotMatch(body, /_arm64-setup\.exe|_amd64\.deb|_universal\.dmg/)
	} finally {
		rmSync(directory, { recursive: true, force: true })
	}
})

test('Windows ARM uses a Java 17 x64 build tool while Rust still targets ARM64', () => {
	for (const file of ['noctrinth-release.yml', 'noctrinth-prerelease.yml']) {
		const release = parse(
			readFileSync(new URL('../.github/workflows/' + file, import.meta.url), 'utf8'),
		)
		const build = Object.values(release.jobs).find((job) => job.strategy?.matrix?.include)
		const java = build.steps.find((step) => step.uses?.startsWith('actions/setup-java@')).with
		assert.equal(String(java['java-version']), '17')
		const expression = java.architecture.slice(3, -2).trim()
		for (const matrix of build.strategy.matrix.include) {
			const architecture = vm.runInNewContext(expression, { matrix })
			assert.equal(architecture, matrix.platform === 'windows-11-arm' ? 'x64' : '')
			if (matrix.platform === 'windows-11-arm')
				assert.match(matrix.args, /--target aarch64-pc-windows-msvc/)
		}
	}
})

test('Windows ARM installs frontend dependencies with Node x64 and matching Corepack shims', () => {
	for (const file of ['noctrinth-release.yml', 'noctrinth-prerelease.yml']) {
		const release = parse(
			readFileSync(new URL('../.github/workflows/' + file, import.meta.url), 'utf8'),
		)
		const build = Object.values(release.jobs).find((job) => job.strategy?.matrix?.include)
		const nodeIndex = build.steps.findIndex((step) => step.uses?.startsWith('actions/setup-node@'))
		const bindIndex = build.steps.findIndex((step) => step.name === 'Bind Corepack to Node x64')
		const installIndex = build.steps.findIndex((step) => step.name === 'Install dependencies')
		const expression = build.steps[nodeIndex].with.architecture.slice(3, -2).trim()
		for (const matrix of build.strategy.matrix.include)
			assert.equal(
				vm.runInNewContext(expression, { matrix }),
				matrix.platform === 'windows-11-arm' ? 'x64' : '',
			)
		assert.ok(nodeIndex < bindIndex && bindIndex < installIndex)
		assert.equal(build.steps[bindIndex].if, "matrix.platform == 'windows-11-arm'")
		assert.match(build.steps[bindIndex].run, /corepack enable --install-directory/)
	}
})
