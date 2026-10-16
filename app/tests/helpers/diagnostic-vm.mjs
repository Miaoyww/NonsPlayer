import { runInNewContext as run } from 'node:vm'

// Most component tests inject IPC stubs. Keep diagnostics observable without a live Tauri runtime.
export function runInNewContext(code, context = {}, options) {
  const original = context.require
  const diagnostics = []
  const reportCaughtError = (operation, cause) => diagnostics.push({ operation, cause })
  return run(
    code,
    {
      ...context,
      diagnosticErrors: diagnostics,
      require(name) {
        if (name !== '@/lib/diagnostics' && name !== '@app/plugin-sdk') return original?.(name)
        let injected
        try {
          injected = original?.(name)
        } catch {
          // A strict loader may reject dependencies not explicitly stubbed in the test.
        }
        return { reportCaughtError, ...injected }
      }
    },
    options
  )
}
