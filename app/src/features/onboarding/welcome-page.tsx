import { AnimatePresence, motion } from 'motion/react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { ExtensionStore } from '@/features/settings/extensions/extension-store'
import { PersonalizationControls } from '@/features/settings/personalization-controls'
import {
  unavailableReason,
  type StoreEntry
} from '@/features/settings/extensions/extension-store-state'
import { useTheme } from '@/features/settings/use-theme'
import { markWelcomeCompleted } from './welcome-state'

type WelcomeStep = 'adapters' | 'plugins' | 'personalization'

function uninstalledEntry(entry: StoreEntry) {
  return !entry.installedVersion && !entry.bundled && !unavailableReason(entry)
}

const steps: {
  id: WelcomeStep
  eyebrow: string
  title: string
  description: string
}[] = [
  {
    id: 'adapters',
    eyebrow: 'STEP 01 / MUSIC SOURCES',
    title: '安装你的音乐适配器',
    description: '从适配器商店选择音乐来源。安装前会显示包的权限和信任信息。'
  },
  {
    id: 'plugins',
    eyebrow: 'STEP 02 / EXTENSIONS',
    title: '挑选需要的插件',
    description: '插件可以扩展播放器的工作方式。你可以安装多个插件，之后也能在设置中管理。'
  },
  {
    id: 'personalization',
    eyebrow: 'STEP 03 / PERSONALIZATION',
    title: '调整成你的样子',
    description: '设置主题、字体、播放栏和界面密度，所有选择都会立即保存。'
  }
]

export function WelcomePage({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState<WelcomeStep>('adapters')
  const [storeBusy, setStoreBusy] = useState(false)
  const [theme, setTheme] = useTheme()

  const currentIndex = steps.findIndex((item) => item.id === step)
  const current = steps[currentIndex]
  const complete = () => {
    markWelcomeCompleted()
    onComplete()
  }
  const next = () => {
    if (currentIndex === steps.length - 1) complete()
    else setStep(steps[currentIndex + 1].id)
  }
  const previous = () => {
    if (currentIndex > 0) setStep(steps[currentIndex - 1].id)
  }

  return (
    <main className="welcome-page" aria-label="欢迎页">
      <div className="welcome-shell">
        <nav className="welcome-steps" aria-label="欢迎页步骤">
          {steps.map((item, index) => {
            const active = index === currentIndex
            return (
              <button
                key={item.id}
                type="button"
                className={`welcome-step ${active ? 'is-active' : ''}`}
                aria-label={`第 ${index + 1} 步：${item.title}`}
                aria-current={active ? 'step' : undefined}
                disabled={storeBusy}
                onClick={() => setStep(item.id)}
              >
                <span className="welcome-step-marker" />
              </button>
            )
          })}
        </nav>

        <section className="welcome-step-panel" aria-labelledby="welcome-step-title">
          <div className="welcome-panel-heading">
            <div>
              <p className="welcome-section-kicker">{current.eyebrow}</p>
              <h2 id="welcome-step-title">{current.title}</h2>
              <p>{current.description}</p>
            </div>
          </div>

          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={step}
              className="welcome-step-content"
              initial={{ opacity: 0, x: 12 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -12 }}
              transition={{ duration: 0.2, ease: 'easeOut' }}
            >
              {step === 'adapters' ? (
                <ExtensionStore
                  kind="adapter"
                  entriesFilter={uninstalledEntry}
                  onInstalled={() => undefined}
                  onBusyChange={setStoreBusy}
                />
              ) : step === 'plugins' ? (
                <ExtensionStore
                  kind="plugin"
                  entriesFilter={uninstalledEntry}
                  onInstalled={() => undefined}
                  onBusyChange={setStoreBusy}
                />
              ) : (
                <div className="welcome-personalization">
                  <PersonalizationControls theme={theme} onThemeChange={setTheme} />
                </div>
              )}
            </motion.div>
          </AnimatePresence>
        </section>

        <footer className="welcome-footer">
          <Button variant="ghost" disabled={storeBusy} onClick={complete}>
            跳过引导
          </Button>
          <div className="flex items-center gap-2">
            <Button variant="ghost" disabled={currentIndex === 0 || storeBusy} onClick={previous}>
              上一步
            </Button>
            <Button className="gap-2 rounded-full px-5" disabled={storeBusy} onClick={next}>
              {currentIndex === steps.length - 1 ? '完成设置' : '下一步'}
            </Button>
          </div>
        </footer>
      </div>
    </main>
  )
}
