import Markdown, { type Components } from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { reportCaughtError } from '@/lib/diagnostics'
import privacyPolicyMarkdown from '../../../../PRIVACY.md?raw'
import './privacy-policy.css'

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
          reportCaughtError('app/src/features/onboarding/privacy-policy.tsx', cause)
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

export function PrivacyPolicyStep({
  accepted,
  onAcceptedChange
}: {
  accepted: boolean
  onAcceptedChange: (accepted: boolean) => void
}) {
  return (
    <div className="welcome-privacy">
      <div
        className="welcome-privacy-document"
        role="region"
        aria-label="隐私协议全文"
        tabIndex={0}
      >
        <article className="privacy-policy">
          <Markdown remarkPlugins={[remarkGfm]} components={components}>
            {privacyPolicyMarkdown}
          </Markdown>
        </article>
      </div>
      <label className="welcome-privacy-consent">
        <input
          type="checkbox"
          className="mt-0.5 size-4 shrink-0 accent-primary"
          checked={accepted}
          onChange={(event) => onAcceptedChange(event.target.checked)}
        />
        <span>我已阅读并同意《隐私协议》</span>
      </label>
    </div>
  )
}
