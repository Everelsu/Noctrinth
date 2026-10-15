/**
 * What the launcher reads from Noctrinth's site: the news articles shown in the
 * sidebar next to Modrinth's, and the changelog the Changelog settings tab shows.
 */
import { type NewsArticle, useVIntl } from '@modrinth/ui'
import { queryOptions, useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import type { NoctrinthVersionEntry } from '@/helpers/noctrinth-changelog'
import { proxiedFetch } from '@/helpers/proxy-fetch'

const CHANGELOG_SITE = 'https://everelsu.github.io/Noctrinth/'
const CHANGELOG_FEED = `${CHANGELOG_SITE}changelog.json`
const NEWS_FEED = `${CHANGELOG_SITE}news/feed/articles.json`

function isEntry(value: unknown): value is NoctrinthVersionEntry {
	const entry = value as Partial<NoctrinthVersionEntry> | null
	return (
		typeof entry?.version === 'string' &&
		typeof entry.date === 'string' &&
		typeof entry.body === 'string'
	)
}

/** The changelog as it stands on the site, which is newer than this build's whenever a release has come out since. */
export function noctrinthChangelogFeedQueryOptions() {
	return queryOptions({
		queryKey: ['noctrinth-changelog-feed'],
		queryFn: async () => {
			const response = await proxiedFetch(CHANGELOG_FEED, { connectTimeout: 8000 })
			if (!response.ok) throw new Error(`HTTP ${response.status}`)
			const entries: unknown = await response.json()
			if (!Array.isArray(entries) || !entries.every(isEntry) || entries.length === 0) {
				throw new Error('The changelog feed is not a list of entries')
			}
			return entries as NoctrinthVersionEntry[]
		},
		staleTime: 10 * 60 * 1000,
		retry: 1,
	})
}

type FeedArticle = Omit<NewsArticle, 'path'> & {
	link: string
	title_ru?: string
	summary_ru?: string
}

/**
 * Noctrinth's news articles, from the site's feed: the same shape as
 * modrinth.com/news/feed/articles.json, plus a Russian title and summary.
 */
export function noctrinthNewsQueryOptions() {
	return queryOptions({
		queryKey: ['news', 'noctrinth-articles'],
		queryFn: async (): Promise<FeedArticle[]> => {
			const response = await proxiedFetch(NEWS_FEED, { connectTimeout: 8000 })
			if (!response.ok) throw new Error(`HTTP ${response.status}`)
			const feed: { articles?: FeedArticle[] } | null = await response.json()
			return feed?.articles ?? []
		},
		staleTime: 10 * 60 * 1000,
		retry: 1,
		refetchOnWindowFocus: false,
	})
}

/** Those articles as news cards, in the launcher's language where they have it. */
export function useNoctrinthNews() {
	const { locale } = useVIntl()
	const feed = useQuery(noctrinthNewsQueryOptions())

	return computed<NewsArticle[]>(() => {
		const russian = locale.value.startsWith('ru')
		return (feed.data.value ?? []).map((article) => ({
			path: article.link,
			thumbnail: article.thumbnail,
			title: (russian && article.title_ru) || article.title,
			summary: (russian && article.summary_ru) || article.summary,
			date: article.date,
		}))
	})
}
