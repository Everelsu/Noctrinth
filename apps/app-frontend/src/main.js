import 'floating-vue/dist/style.css'
import 'overlayscrollbars/overlayscrollbars.css'

import { VueScanPlugin } from '@taijased/vue-render-tracker'
import { VueQueryPlugin } from '@tanstack/vue-query'
import FloatingVue from 'floating-vue'
import { createApp } from 'vue'

import App from '@/App.vue'
import { overlayScrollbarsDirective } from '@/directives/overlayScrollbars'
import { installPopperCleanup } from '@/helpers/noctrinth-popper-cleanup'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

const vueScan = new VueScanPlugin({
	enabled: false, // Enable or disable the tracker
	showOverlay: true, // Show overlay to visualize renders
	log: false, // Log render events to the console
	playSound: false, // Play sound on each render
})

let app = createApp(App)

app.use(VueQueryPlugin)
app.use(vueScan)
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

app.mount('#app')
