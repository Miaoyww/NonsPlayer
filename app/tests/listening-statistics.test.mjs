import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

const code = ts.transpileModule(
  readFileSync(
    new URL('../src/features/library/music-library/listening-statistics.ts', import.meta.url),
    'utf8'
  ),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
).outputText

function load() {
  const exports = {}
  runInNewContext(code, { exports })
  return exports
}

test('listening duration is formatted from seconds up to hours and minutes', () => {
  const { formatListeningDuration } = load()
  assert.equal(formatListeningDuration(0), '0 秒')
  assert.equal(formatListeningDuration(59), '59 秒')
  assert.equal(formatListeningDuration(60), '1 分钟')
  assert.equal(formatListeningDuration(3599), '59 分钟')
  assert.equal(formatListeningDuration(3600), '1 小时')
  assert.equal(formatListeningDuration(3600 + 25 * 60), '1 小时 25 分')
  assert.equal(formatListeningDuration(-10), '0 秒')
  assert.equal(formatListeningDuration(3600.9), '1 小时')
})

test('listening counts drop fractions and group thousands', () => {
  const { formatListeningCount } = load()
  assert.equal(formatListeningCount(0), '0')
  assert.equal(formatListeningCount(12.8), '12')
  assert.equal(formatListeningCount(12345), '12,345')
  assert.equal(formatListeningCount(-3), '0')
})
