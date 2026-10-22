import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function load(path) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    { exports, setTimeout, clearTimeout }
  )
  return exports
}
test('recovery targets the failed playback instance and login is offered for explicit expiry only', () => {
  const { failedPlayback, samePlayback } = load('../src/features/playback/recovery.ts')
  const track = { key: 'qq:1', title: 'Song', source: { kind: 'online' } }
  const state = {
    error: '账号会话已过期',
    status: 'stopped',
    index: 0,
    queue: [track],
    revision: 4
  }
  assert.equal(failedPlayback(state).login, true)
  assert.equal(failedPlayback({ ...state, error: '网络超时' }).login, false)
  assert.equal(failedPlayback({ ...state, status: 'loading' }), null)
  assert.equal(samePlayback(state, 4, track), true)
  assert.equal(samePlayback({ ...state, revision: 5 }, 4, track), false)
  assert.equal(samePlayback({ ...state, queue: [{ ...track, key: 'qq:2' }] }, 4, track), false)
})
test('actionable failures stay available and retiring one playback does not clear unrelated errors', () => {
  let scheduled = 0
  const store = load('../src/features/playback/error-notices.ts').createErrorNotices(() => {
    scheduled++
  })
  store.push('其他页面错误')
  store.push('播放失败', [{ label: '重试', run() {} }], 'playback-recovery')
  assert.equal(scheduled, 1)
  assert.equal(store.getSnapshot().length, 2)
  store.clearGroup('playback-recovery')
  assert.equal(store.getSnapshot().length, 1)
  assert.equal(store.getSnapshot()[0].message, '其他页面错误')
})
