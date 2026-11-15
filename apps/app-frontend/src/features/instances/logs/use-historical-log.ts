import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { computed, type Ref } from 'vue'

import {
	analyzeHistoricalLog,
	getHistoricalPage,
	getHistoricalText,
	type HistoricalLog,
	type HistoricalLogCursor,
} from '@/platform/app-lib/logs/commands'

export function useHistoricalLog(instanceId: Ref<string>, log: Ref<HistoricalLog | undefined>) {
	const enabled = computed(() => !!log.value && !log.value.live)
	const key = computed(() => [
		'instances',
		'summary',
		instanceId.value,
		'logs',
		log.value?.log_type,
		log.value?.filename,
	])
	const pages = useInfiniteQuery({
		queryKey: computed(() => [...key.value, 'pages']),
		queryFn: ({ pageParam }) => getHistoricalPage(instanceId.value, log.value!, pageParam),
		initialPageParam: null as HistoricalLogCursor | null,
		getNextPageParam: (page) => page.cursor,
		enabled,
		networkMode: 'always',
		staleTime: Infinity,
		gcTime: 0,
		refetchOnWindowFocus: false,
		retry: false,
	})
	const full = useQuery({
		queryKey: computed(() => [...key.value, 'full']),
		queryFn: () => getHistoricalText(instanceId.value, log.value!),
		enabled: false,
		networkMode: 'always',
		gcTime: 0,
		retry: false,
	})
	const diagnosis = useQuery({
		queryKey: computed(() => [...key.value, 'diagnosis']),
		queryFn: () => analyzeHistoricalLog(instanceId.value, log.value!),
		enabled,
		networkMode: 'always',
		staleTime: Infinity,
		gcTime: 0,
		retry: false,
	})
	const output = computed(() => {
		if (!enabled.value) return ''
		if (full.data.value !== undefined) return full.data.value
		const chunks = pages.data.value?.pages ?? []
		const resetIndex = chunks.findLastIndex((page) => page.reset)
		return chunks
			.slice(Math.max(0, resetIndex))
			.toReversed()
			.map((page) => page.output)
			.join('')
	})
	return {
		output,
		diagnosis: diagnosis.data,
		error: pages.error,
		loading: computed(() => enabled.value && (pages.isPending.value || full.isFetching.value)),
		hasEarlier: computed(
			() => enabled.value && full.data.value === undefined && pages.hasNextPage.value,
		),
		loadingEarlier: pages.isFetchingNextPage,
		loadEarlier: async () => {
			if (!pages.isFetching.value && pages.hasNextPage.value) {
				const result = await pages.fetchNextPage()
				if (result.error) throw result.error
			}
		},
		loadAll: async () => {
			if (!enabled.value || full.data.value !== undefined) return
			const result = await full.refetch()
			if (result.error) throw result.error
		},
	}
}
