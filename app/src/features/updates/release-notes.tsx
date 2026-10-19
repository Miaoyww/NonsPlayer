import Markdown, { type Components } from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { reportCaughtError } from '@/lib/diagnostics'
import './release-notes.css'

const components: Components = {
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(event) => {
        if (!href || !isTauri()) return
        event.preventDefault()
        void openUrl(href).catch((cause) => {
          reportCaughtError('app/src/features/updates/release-notes.tsx', cause)
        })
      }}
    >
      {children}
    </a>
  ),
  table: ({ children }) => (
    <div className="overflow-x-auto">
      <table>{children}</table>
    </div>
  )
}

export function ReleaseNotes({ notes }: { notes: string }) {
  return (
    <section
      aria-label="更新说明"
      tabIndex={0}
      className="release-notes min-h-0 overflow-y-auto overscroll-contain pr-3 text-sm leading-7 [overflow-wrap:anywhere]"
    >
      <Markdown remarkPlugins={[remarkGfm]} components={components}>
        {notes}
      </Markdown>
    </section>
  )
}
