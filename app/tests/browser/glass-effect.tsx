import { createRoot } from 'react-dom/client'
import { useEffect, useState } from 'react'
import { PersonalizationControls } from '../../src/features/settings/appearance/personalization-controls'
import { initializeGlassEffect } from '../../src/features/settings/appearance/use-glass-effect'
import type { Theme } from '../../src/features/settings/appearance/use-theme'
import '../../src/styles/globals.css'

initializeGlassEffect()

function Harness() {
  const [theme, setTheme] = useState<Theme>('light')
  useEffect(() => {
    document.documentElement.classList.toggle('dark', theme === 'dark')
  }, [theme])
  return (
    <>
      <div className="glass-surface fixed right-4 top-4 size-24" data-testid="surface" />
      <PersonalizationControls theme={theme} onThemeChange={setTheme} />
    </>
  )
}

createRoot(document.getElementById('root') as HTMLElement).render(<Harness />)
