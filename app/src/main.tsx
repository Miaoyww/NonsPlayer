import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import '@/styles/globals.css'
import { initializeFontSettings } from '@/features/settings/font-settings'
import { installPluginBridge } from '@/plugins/host'

initializeFontSettings()
installPluginBridge()

// Release WebViews are application surfaces, not browser windows.
if (!import.meta.env.DEV) {
  window.addEventListener(
    'keydown',
    (event) => {
      const key = event.key.toLowerCase()
      const command = event.ctrlKey || event.metaKey
      const browserShortcut =
        key === 'f12' ||
        key === 'f5' ||
        (command &&
          !event.altKey &&
          ['f', 'g', 'p', 'r', 's', 'u', '+', '=', '-', '0'].includes(key)) ||
        (command && (event.shiftKey || event.altKey) && ['i', 'j', 'c'].includes(key))
      if (!browserShortcut) return
      event.preventDefault()
      // Ctrl/Cmd+F is handled by the workspace list finder in both dev and release.
      if (command && key === 'f' && !event.altKey && !event.shiftKey) return
      event.stopImmediatePropagation()
    },
    { capture: true }
  )
}

// Let custom menus handle the event first, then suppress the WebView default menu.
window.addEventListener('contextmenu', (event) => {
  event.preventDefault()
})

// Keep Tab from moving focus or triggering component-level navigation.
window.addEventListener(
  'keydown',
  (event) => {
    if (event.key !== 'Tab') return
    event.preventDefault()
    event.stopImmediatePropagation()
  },
  { capture: true }
)

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
