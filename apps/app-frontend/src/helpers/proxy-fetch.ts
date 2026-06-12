/**
 * Drop-in replacement for `fetch` from `@tauri-apps/plugin-http` that routes
 * requests through the proxy configured in Settings → Resource management
 * (`proxy_url`). The proxy is read once per app session — like the Rust side,
 * changing it requires an app restart.
 */
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { get as getSettings } from './settings'

type TauriFetchInit = Parameters<typeof tauriFetch>[1] & {
	proxy?: { all: string }
}

let proxyUrlPromise: Promise<string | null> | null = null

function getProxyUrl(): Promise<string | null> {
	if (!proxyUrlPromise) {
		proxyUrlPromise = getSettings()
			.then((settings) => (settings as { proxy_url?: string | null }).proxy_url || null)
			.catch(() => null)
	}
	return proxyUrlPromise
}

export async function proxiedFetch(
	input: Parameters<typeof tauriFetch>[0],
	init?: TauriFetchInit,
): Promise<Response> {
	const proxyUrl = await getProxyUrl()
	if (proxyUrl) {
		init = { ...init, proxy: { all: proxyUrl } }
	}
	return tauriFetch(input, init)
}
