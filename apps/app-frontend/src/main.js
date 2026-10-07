import 'overlayscrollbars/overlayscrollbars.css'
import '@/assets/stylesheets/noctrinth-motion.css'

import { installTooltipDirective } from '@modrinth/ui'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { createApp } from 'vue'

import App from '@/App.vue'
import { overlayScrollbarsDirective } from '@/directives/overlayScrollbars'
import { setupErrorReporting } from '@/helpers/error-reporting'
import { installTooltipCleanup } from '@/helpers/noctrinth-tooltip-cleanup'
import { debugStartup, traceStartupStep } from '@/helpers/startup-debug'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

debugStartup('Frontend entry module evaluated')
const app = createApp(App)
setupErrorReporting(app, router)

// Every query refetching whenever the window gets focus is a website's habit.
// Here focus comes and goes each time someone switches between the launcher
// and the game, and a round of IPC calls and API requests on every switch is
// what the launcher stuttered on. What changes is invalidated by events.
app.use(VueQueryPlugin, {
	queryClientConfig: { defaultOptions: { queries: { refetchOnWindowFocus: false } } },
})
app.use(router)
if (import.meta.env.DEV) {
	void traceStartupStep('Initial router readiness', () => router.isReady()).catch(() => {})
}
app.use(i18nPlugin)
app.use(i18nDebugPlugin)
installTooltipDirective(app)

// Tooltips that were never told to hide, because the window lost focus or the
// pointer left it while one was up.
installTooltipCleanup(router)
app.directive('overlay-scrollbars', overlayScrollbarsDirective)

async function mount() {
	if (import.meta.env.DEV && import.meta.env.VITE_VUE_SCAN === 'true') {
		const { VueScanPlugin } = await traceStartupStep(
			'Load Vue render tracker',
			() => import('@taijased/vue-render-tracker'),
		)
		app.use(new VueScanPlugin({ enabled: true, showOverlay: true, log: false, playSound: false }))
	}
	debugStartup('Vue mount started')
	app.mount('#app')
	debugStartup('Vue mount completed')
}

void mount()
