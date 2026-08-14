/**
 * Runs `tauri dev` against a data directory of its own, so a dev build never
 * migrates the database the installed app is using.
 *
 * `THESEUS_CONFIG_DIR` replaces the launcher's whole base directory — app.db,
 * settings, logs, instances, metadata, Java and caches all move with it — so
 * the dev build starts empty and leaves the real install untouched. An already
 * exported THESEUS_CONFIG_DIR wins, for pointing at a copy of real data.
 *
 * It also clears a stale Vite server off the dev port first: a `tauri dev` that
 * fails after its beforeDevCommand started leaves the frontend running, and
 * every later run then dies with "Port 1420 is already in use".
 *
 *   node scripts/dev-isolated.mjs [--keep-port] [-- <extra tauri args>]
 */
import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createConnection } from 'node:net'
import { homedir, tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const appDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repoRoot = resolve(appDir, '..', '..')

function defaultDataDir() {
	// Mirrors the launcher's own base directory, with a `-dev` suffix, so dev
	// data sits beside the real thing instead of inside the repository.
	const identifier = 'com.noctrinth.app-dev'
	if (process.platform === 'win32') {
		return join(process.env.APPDATA ?? join(homedir(), 'AppData', 'Roaming'), identifier)
	}
	if (process.platform === 'darwin') {
		return join(homedir(), 'Library', 'Application Support', identifier)
	}
	return join(process.env.XDG_DATA_HOME ?? join(homedir(), '.local', 'share'), identifier)
}

function tauriConfig() {
	try {
		return JSON.parse(readFileSync(join(appDir, 'tauri.conf.json'), 'utf8'))
	} catch {
		return null
	}
}

function devPort() {
	// Keep in step with tauri.conf.json rather than hard-coding 1420 twice.
	const port = Number(new URL(tauriConfig()?.build?.devUrl ?? '').port)
	return Number.isFinite(port) && port > 0 ? port : 1420
}

/**
 * tauri.conf.json points `devUrl` at `http://localhost:<port>`, and Vite binds
 * to whichever loopback Node hands it. On a machine where `localhost` resolves
 * to ::1 those two can land on different stacks, and Tauri then waits forever
 * for a dev server that is answering perfectly well on the other one. Pinning
 * both sides to 127.0.0.1 removes the guesswork; the override is passed to the
 * CLI so upstream's config file stays untouched.
 */
function loopbackOverride(port) {
	const config = tauriConfig()
	const before =
		config?.build?.beforeDevCommand ?? 'pnpm turbo run dev --filter=@modrinth/app-frontend'
	const override = {
		build: {
			devUrl: `http://127.0.0.1:${port}`,
			beforeDevCommand: `${before} -- --host 127.0.0.1 --port ${port} --strictPort`,
		},
	}

	const path = join(tmpdir(), `noctrinth-dev-override-${port}.json`)
	writeFileSync(path, JSON.stringify(override), 'utf8')
	return path
}

function portInUse(port) {
	return new Promise((done) => {
		const socket = createConnection({ port, host: '127.0.0.1' })
		socket.once('connect', () => {
			socket.destroy()
			done(true)
		})
		socket.once('error', () => done(false))
		socket.setTimeout(1000, () => {
			socket.destroy()
			done(false)
		})
	})
}

function run(command, args) {
	return execFileSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
}

/** PIDs listening on `port`, best-effort and per-platform. */
function listenerPids(port) {
	try {
		if (process.platform === 'win32') {
			return [
				...new Set(
					run('netstat', ['-ano', '-p', 'TCP'])
						.split('\n')
						.filter((line) => line.includes(`:${port} `) && line.includes('LISTENING'))
						.map((line) => line.trim().split(/\s+/).pop())
						.filter((pid) => pid && pid !== '0'),
				),
			]
		}
		return [...new Set(run('lsof', ['-ti', `tcp:${port}`, '-sTCP:LISTEN']).split('\n'))].filter(
			Boolean,
		)
	} catch {
		return []
	}
}

/** The command line of `pid`, or '' when it cannot be read. */
function commandLine(pid) {
	try {
		if (process.platform === 'win32') {
			return run('powershell', [
				'-NoProfile',
				'-Command',
				`(Get-CimInstance Win32_Process -Filter "ProcessId = ${pid}").CommandLine`,
			])
		}
		return run('ps', ['-o', 'command=', '-p', pid])
	} catch {
		return ''
	}
}

async function freePort(port) {
	if (!(await portInUse(port))) return true

	for (const pid of listenerPids(port)) {
		const command = commandLine(pid)
		// Only ever kill this repository's own frontend server. Anything else on
		// the port is someone's real work and is left alone.
		const isOurVite = /vite/i.test(command) && command.includes(repoRoot)
		if (!isOurVite) {
			console.error(
				`Port ${port} is held by PID ${pid}, which is not this repo's dev server:\n  ${command.trim()}\nStop it yourself, or pass --keep-port to try anyway.`,
			)
			return false
		}

		console.log(`Clearing stale dev server on port ${port} (PID ${pid})`)
		try {
			if (process.platform === 'win32') run('taskkill', ['/PID', pid, '/T', '/F'])
			else process.kill(Number(pid), 'SIGKILL')
		} catch (error) {
			console.error(`Could not stop PID ${pid}: ${error.message}`)
			return false
		}
	}

	await new Promise((done) => setTimeout(done, 700))
	return !(await portInUse(port))
}

const argv = process.argv.slice(2)
const keepPort = argv.includes('--keep-port')
const tauriArgs = argv.filter((arg) => arg !== '--keep-port')

const port = devPort()
if (!keepPort && !(await freePort(port))) process.exit(1)

const dataDir = process.env.THESEUS_CONFIG_DIR ?? defaultDataDir()
if (!existsSync(dataDir)) mkdirSync(dataDir, { recursive: true })

console.log(`Launcher data: ${dataDir}`)
console.log(
	'Deep links: this run claims noctrinth:// and modrinth:// for the dev binary. Start the installed app once afterwards to hand them back.',
)

const child = spawn(
	'pnpm',
	[
		'exec',
		'tauri',
		'dev',
		// Keep in step with the plain `dev` script — the frontend's app-event
		// listeners are gated behind this feature.
		'--features',
		'export-app-events',
		'--config',
		loopbackOverride(port),
		...tauriArgs,
	],
	{
		cwd: appDir,
		stdio: 'inherit',
		shell: process.platform === 'win32',
		env: {
			...process.env,
			THESEUS_CONFIG_DIR: dataDir,
			// Keep dev's pre-migration snapshots out of the installed app's backups.
			THESEUS_DB_BACKUP_DIR: process.env.THESEUS_DB_BACKUP_DIR ?? join(dataDir, 'db-backups'),
		},
	},
)

child.on('exit', (code, signal) => process.exit(signal ? 1 : (code ?? 0)))
