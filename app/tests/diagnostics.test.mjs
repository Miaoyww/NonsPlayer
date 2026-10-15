import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInThisContext } from 'node:vm'
import ts from 'typescript'

function harness(failLog = false) {
  const calls = [],
    consoleLogs = [],
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
  return { ...exports, calls, consoleLogs, handlers }
}

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
    'failed\nCookie: SECRET'
  ])
    assert.ok(!app.redactDiagnostic(text).includes('SECRET'))
  assert.ok(!app.redactDiagnostic('bad\nforged log').includes('\n'))
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
