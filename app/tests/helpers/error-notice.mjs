import { createElement } from 'react'

// Other feature tests inspect reported messages while testing their own state machines.
// The real reporter, shared store and portal are exercised in error-notice.test.mjs.
export const errorNoticeModule = {
  ErrorNotice: ({ error }) =>
    error ? createElement('p', { role: 'alert', 'data-error-notice': true }, error) : null,
  ErrorNoticeViewport: () => null
}
