import { createRoot } from 'react-dom/client'
import { WelcomePage } from '../../src/features/onboarding/welcome-page'
import '../../src/styles/globals.css'

const entries = (kind: 'adapter' | 'plugin') =>
  Array.from({ length: 55 }, (_, index) => ({
    id: `${kind}-${index}`,
    name: `${kind === 'adapter' ? '音乐来源' : '音乐工具'} ${index + 1}`,
    kind,
    version: '1.0.0',
    description: '选择并安装需要的音乐扩展。',
    authors: [{ name: 'NonsPlayer' }],
    compatible: true,
    bundled: false,
    updateAvailable: false,
    artifact: {
      url: 'https://github.com/NonsPlayer/example/releases/download/v1.0.0/example.zip',
      size: 1024,
      sha256: '0'.repeat(64)
    }
  }))

const host = globalThis as unknown as {
  isTauri: boolean
  __TAURI_INTERNALS__: {
    invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>
  }
}
host.isTauri = true
host.__TAURI_INTERNALS__ = {
  async invoke(command, args) {
    if (command === 'extension_store_index')
      return {
        source: args?.source ?? 'cdn',
        url: 'https://nons-plugn.miaoyww.top/index.json',
        generatedAt: '2026-10-30T12:00:00Z',
        plugins: entries('plugin'),
        adapters: entries('adapter')
      }
    return null
  }
}

createRoot(document.getElementById('root')!).render(
  <div className="app-shell welcome-app-shell relative flex h-dvh flex-col overflow-hidden text-foreground">
    <WelcomePage onComplete={() => undefined} />
  </div>
)
