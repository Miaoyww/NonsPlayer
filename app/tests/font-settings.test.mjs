import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const source = readFileSync(
  new URL('../src/features/settings/font-settings.ts', import.meta.url),
  'utf8'
)
function session(saved, storageFailure = false) {
  const values = new Map(saved === undefined ? [] : [['nons-font-settings', saved]])
  const styles = new Map()
  const exports = {}
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      document: {
        documentElement: { style: { setProperty: (key, value) => styles.set(key, value) } }
      },
      localStorage: {
        getItem: (key) => {
          if (storageFailure) throw Error('unavailable')
          return values.get(key) ?? null
        },
        setItem: (key, value) => {
          if (storageFailure) throw Error('unavailable')
          values.set(key, value)
        }
      }
    }
  )
  exports.initializeFontSettings()
  return { ...exports, values, styles }
}

test('app and lyric fonts remain independent, persist and restore before rendering', () => {
  const state = session()
  let notifications = 0
  const stop = state.subscribeFontSettings(() => notifications++)
  state.setFont('app', 'Georgia')
  assert.match(state.styles.get('--nons-app-font'), /^"Georgia",/)
  assert.equal(state.styles.get('--nons-lyrics-font'), 'var(--nons-app-font)')
  state.setFont('lyrics', 'Microsoft YaHei')
  state.setFont('app', 'Arial')
  assert.match(state.styles.get('--nons-lyrics-font'), /^"Microsoft YaHei",/)
  const restarted = session(state.values.get('nons-font-settings'))
  assert.equal(
    JSON.stringify(restarted.getFontSettings()),
    '{"app":"Arial","lyrics":"Microsoft YaHei"}'
  )
  assert.equal(restarted.styles.get('--nons-lyrics-font'), state.styles.get('--nons-lyrics-font'))
  state.setFont('lyrics', '')
  assert.equal(state.styles.get('--nons-lyrics-font'), 'var(--nons-app-font)')
  assert.equal(notifications, 4)
  stop()
  state.setFont('app', '')
  assert.equal(notifications, 4)
  assert.equal(state.styles.get('--nons-app-font'), state.defaultFontStack)
})

test('corrupt preferences and unavailable storage preserve usable defaults and session changes', () => {
  for (const saved of ['invalid json', 'null', '{"app":42,"lyrics":"bad\\nfont"}']) {
    const state = session(saved)
    assert.equal(JSON.stringify(state.getFontSettings()), '{"app":"","lyrics":""}')
  }
  const state = session(undefined, true)
  state.setFont('app', 'Georgia')
  assert.match(state.styles.get('--nons-app-font'), /^"Georgia",/)
  state.setFont('app', 'bad\nfont')
  assert.equal(state.getFontSettings().app, 'Georgia')
  assert.equal(state.fontStack('Family "One"\\Two').split(', ')[0], '"Family \\"One\\"\\\\Two"')
})
