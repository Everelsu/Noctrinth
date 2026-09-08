import 'floating-vue/dist/style.css'
import 'overlayscrollbars/overlayscrollbars.css'

import { VueQueryPlugin } from '@tanstack/vue-query'
import FloatingVue from 'floating-vue'
import { createApp } from 'vue'

import App from '@/App.vue'
import { overlayScrollbarsDirective } from '@/directives/overlayScrollbars'
import { setupErrorReporting } from '@/helpers/error-reporting'
import { installPopperCleanup } from '@/helpers/noctrinth-popper-cleanup'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

const app = createApp(App)
setupErrorReporting(app, router)

app.use(VueQueryPlugin)
app.use(router)
app.use(FloatingVue, {
	themes: {
		'ribbit-popout': {
			$extend: 'dropdown',
			placement: 'bottom-end',
			instantMove: true,
			distance: 8,
		},
		'dismissable-prompt': {
			$extend: 'dropdown',
			placement: 'bottom-start',
		},
	},
})
app.use(i18nPlugin)
app.use(i18nDebugPlugin)

// Tooltips that were never told to hide, because what they belonged to went
// away while the pointer was still on it.
installPopperCleanup(router)
app.directive('overlay-scrollbars', overlayScrollbarsDirective)

async function mount() {
	if (import.meta.env.DEV && import.meta.env.VITE_VUE_SCAN === 'true') {
		const { VueScanPlugin } = await import('@taijased/vue-render-tracker')
		app.use(new VueScanPlugin({ enabled: true, showOverlay: true, log: false, playSound: false }))
	}
	app.mount('#app')
}

void mount()
