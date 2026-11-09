import { createRoot } from 'react-dom/client'
import { NetworkServicePage } from '../../src/features/settings/network-service/network-service-page'
import '../../src/styles/globals.css'
import { calls, copied, emitStatus, opened } from './network-service-stubs'

declare global {
  interface Window {
    networkServiceFixture: {
      calls: typeof calls
      copied: typeof copied
      opened: typeof opened
      emitStatus: typeof emitStatus
    }
  }
}

window.networkServiceFixture = { calls, copied, opened, emitStatus }
createRoot(document.getElementById('root')!).render(<NetworkServicePage />)
