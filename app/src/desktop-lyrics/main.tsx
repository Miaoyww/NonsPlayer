import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { DesktopLyricsApp } from '@/features/desktop-lyrics/desktop-lyrics-app'
import '@/features/desktop-lyrics/desktop-lyrics.css'
import { initializeFontSettings } from '@/features/settings/appearance/font-settings'

document.documentElement.style.background = 'transparent'
document.body.style.background = 'transparent'
document.documentElement.dataset.desktopLyrics = 'true'
initializeFontSettings()

createRoot(document.getElementById('root') as HTMLElement).render(
  <StrictMode>
    <DesktopLyricsApp />
  </StrictMode>
)
