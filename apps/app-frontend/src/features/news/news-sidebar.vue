<script setup lang="ts">
import { NewspaperIcon } from '@modrinth/assets'
import { ButtonLink, defineMessages, NewsArticleCard, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, watch } from 'vue'

import { NOCTRINTH_NEWS_URL, useNoctrinthNews } from './noctrinth-news'
import { newsArticlesQueryOptions } from './queries'

const { formatMessage } = useVIntl()
const messages = defineMessages({
	news: {
		id: 'app.news.title',
		defaultMessage: 'News',
	},
	noctrinthNews: {
		id: 'noctrinth.news.noctrinth-news',
		defaultMessage: 'Noctrinth News',
	},
	modrinthNews: {
		id: 'noctrinth.news.modrinth-news',
		defaultMessage: 'Modrinth News',
	},
})

const { data: modrinthNews, error } = useQuery(newsArticlesQueryOptions())
// Noctrinth: its own articles mixed in with Modrinth's, newest first.
const noctrinthNews = useNoctrinthNews()
const news = computed(() =>
	[...noctrinthNews.value, ...(modrinthNews.value ?? [])]
		.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
		.slice(0, 4),
)

watch(error, (error) => {
	if (error) console.error('Failed to fetch news articles', error)
})
</script>

<template>
	<div class="p-4 flex flex-col items-center">
		<h3 class="text-base mb-4 text-primary font-medium m-0 text-left w-full">
			{{ formatMessage(messages.news) }}
		</h3>
		<div class="space-y-4 flex flex-col items-center w-full">
			<NewsArticleCard v-for="item in news" :key="item.path" :article="item" />
		</div>
		<div class="my-4 flex w-full flex-col gap-2">
			<ButtonLink
				type="colored"
				color="brand"
				size="xl"
				:href="NOCTRINTH_NEWS_URL"
				target="_blank"
				class="justify-center"
			>
				<NewspaperIcon />
				{{ formatMessage(messages.noctrinthNews) }}
			</ButtonLink>
			<ButtonLink
				type="quiet"
				href="https://modrinth.com/news"
				target="_blank"
				class="justify-center"
			>
				{{ formatMessage(messages.modrinthNews) }}
			</ButtonLink>
		</div>
	</div>
</template>
