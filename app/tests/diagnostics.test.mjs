import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInThisContext } from 'node:vm'
import ts from 'typescript'

function harness(failLog = false, holdLogs = false) {
  const calls = [],
    consoleLogs = [],
    completions = [],
    handlers = new Map()
  const output = ts.transpileModule(
    readFileSync(new URL('../src/lib/diagnostics.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText
  const exports = {}
  runInThisContext(`(function(exports, require, console, window) { ${output}\n })`)(
    exports,
    () => ({
      listen: async (name, callback) => {
        handlers.set(name, callback)
        return () => {}
      },
      isTauri: () => true,
      invoke: async (command, args) => {
        calls.push({ command, args })
        if (command === 'diagnostic_report') {
          if (failLog) throw new Error('disk unavailable')
          if (holdLogs) await new Promise((resolve) => completions.push(resolve))
          return
        }
        throw { code: 'internal', message: 'database table missing', stage: 'save_queue_tracks' }
      }
    }),
    Object.fromEntries(
      ['info', 'warn', 'error'].map((level) => [level, (...args) => consoleLogs.push(args)])
    ),
    { addEventListener: (name, callback) => handlers.set(name, callback) }
  )
  return { ...exports, calls, consoleLogs, handlers, completions }
}

test('caught errors keep the original cause, deduplicate recovery boundaries and classify expected retries', async () => {
  const app = harness()
  let original
  try {
    await app.diagnosticInvoke('music_query')
  } catch (cause) {
    original = cause
    app.reportCaughtError('query.recover', cause)
  }
  assert.equal(original.message, 'database table missing')
  assert.equal(app.calls.filter((call) => call.command === 'diagnostic_report').length, 1)
  for (const code of [
    'staleContext',
    'cancelled',
    'cacheInvalidated',
    'writeRevisionChanged',
    'emptyLyrics'
  ]) {
    app.reportCaughtError('query.retry', { code, message: 'expected lifecycle change' })
    assert.equal(app.calls.at(-1).args.entry.level, 'info')
  }
  const cause = new Error('fallback failed')
  app.reportCaughtError('fallback', cause)
  app.reportCaughtError('page', cause)
  assert.equal(app.calls.at(-1).args.entry.level, 'error')
  assert.match(app.calls.at(-1).args.entry.message, /fallback failed/)
})

test('a burst stays bounded and records an overflow count after the sink recovers', async () => {
  const app = harness(false, true)
  for (let i = 0; i < 100; i++) {
    app.reportCaughtError('burst', new Error(`failure ${i}`))
  }
  assert.ok(app.completions.length <= 16)
  for (let round = 0; round < 10; round++) {
    app.completions.splice(0).forEach((resolve) => resolve())
    await new Promise((resolve) => setTimeout(resolve, 140))
  }
  const entries = app.calls
    .filter((call) => call.command === 'diagnostic_report')
    .map((call) => call.args.entry)
  const summary = entries.find((entry) => entry.code === 'diagnosticOverflow')
  assert.ok(summary, 'overflow must leave a file diagnostic')
  const dropped = Number(summary.message.match(/dropped=(\d+)/)?.[1])
  assert.ok(dropped > 0)
  assert.equal(entries.filter((entry) => entry.operation === 'burst').length + dropped, 100)
})

test('handled music failures log operation, source, duration and code without request payloads', async () => {
  const app = harness()
  await assert.rejects(
    app.diagnosticInvoke('music_query', {
      source: 'netease',
      request: { operation: 'tracks', cookie: 'SECRET' }
    }),
    { code: 'internal' }
  )
  const report = app.calls.find((call) => call.command === 'diagnostic_report').args.entry
  assert.equal(report.command, 'music_query')
  assert.equal(report.source, 'netease')
  assert.equal(report.operation, 'tracks')
  assert.equal(report.code, 'internal')
  assert.equal(report.message, 'database table missing')
  assert.equal(report.stage, 'save_queue_tracks')
  assert.equal(report.level, 'error')
  assert.ok(report.elapsedMs >= 0)
  assert.ok(!JSON.stringify(report).includes('SECRET'))
  assert.equal(app.consoleLogs.length, 1)
})

test('redaction removes URLs, credential tails, JSON payloads and log line injection', () => {
  const app = harness()
  for (const text of [
    'failed https://u:SECRET@example.com/path?token=SECRET',
    'request failed Cookie: session=SECRET; more=SECRET',
    'request failed Authorization: Bearer SECRET',
    'response {"data":"SECRET"}',
    'failed token=SECRET',
    'failed credential: SECRET',
    'failed\nCookie: SECRET',
    'invalid type: string "SECRET", expected a boolean at line 1 column 8',
    'invalid value: string "SECRET\\"more", expected a number',
    'unknown variant "SECRET", expected `ok` or `error`',
    'invalid type: string "SECRET'
  ])
    assert.ok(!app.redactDiagnostic(text).includes('SECRET'))
  assert.ok(!app.redactDiagnostic('bad\nforged log').includes('\n'))
  assert.equal(
    app.redactDiagnostic('credential store denied access'),
    'credential store denied access'
  )
  assert.match(
    app.redactDiagnostic('invalid type: string "SECRET", expected a boolean'),
    /expected a boolean/
  )
})

test('unhandled errors are reported and a failed log sink never rejects or recurses', async () => {
  const app = harness(true)
  app.installDiagnostics()
  app.handlers.get('error')({ error: new Error('render failed'), message: 'render failed' })
  app.handlers.get('unhandledrejection')({ reason: { code: 'staleContext' } })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(app.calls.length, 2)
  assert.equal(app.calls[1].args.entry.level, 'info')
  assert.equal(app.calls[1].args.entry.code, 'staleContext')
  assert.equal(app.consoleLogs.length, 4)
})

test('native reasons reach the browser console without being submitted back to the host', async () => {
  const app = harness()
  app.installDiagnostics()
  app.handlers.get('log://log')({
    payload: { level: 5, message: 'save_queue_tracks: database table missing' }
  })
  assert.equal(app.calls.length, 0)
  assert.match(app.consoleLogs[0][1], /database table missing/)
  app.handlers.get('log://log')({
    payload: { level: 5, message: '2026-10-15 ERROR nons::webview command=music_queue_tracks' }
  })
  assert.equal(app.consoleLogs.length, 1)
})
