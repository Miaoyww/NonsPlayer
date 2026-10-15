import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'
import { coverImage } from './helpers/cover-image.mjs'
const code = ts.transpileModule(
  readFileSync(new URL('../src/features/library/library-preview.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
).outputText
function harness(raw) {
  let now = 1000000000
  const storage = {
    raw,
    getItem() {
      return this.raw
    },
    setItem(_key, value) {
      this.raw = value
    },
    removeItem() {
      this.raw = null
    }
  }
  const exports = {}
  runInNewContext(code, {
    exports,
    Date: { now: () => now },
    localStorage: storage,
    require: () => coverImage
  })
  return {
    ...exports,
    storage,
    expire() {
      now += 24 * 60 * 60 * 1000 + 1
    },
    restart() {
      return harness(storage.raw)
    }
  }
}
const owner = (id) => JSON.stringify({ source: 'qqmusic', id: String(id) })
const summary = (userId) => ({
  profile: {
    userId: String(userId),
    reference: { source: 'qqmusic', id: String(userId) },
    nickname: 'Saved',
    avatarUrl: ''
  },
  likedPlaylist: null,
  likedTracks: [],
  likedError: null
})
const collections = (name) => ({
  items: [
    {
      reference: { source: 'qqmusic', kind: 'playlist', id: '10' },
      kind: 'playlist',
      id: '10',
      name,
      cover: '',
      subtitle: '',
      trackCount: 5
    }
  ],
  more: false
})
test('library previews survive restart, isolate owners and retain only the latest two accounts', () => {
  const app = harness()
  app.saveLibraryPreview(owner(1), { summary: summary(1) })
  app.saveLibraryPreview(owner(1), { collections: collections('One') })
  const restart = app.restart()
  assert.equal(restart.peekLibraryPreview(owner(1)).collections.items[0].name, 'One')
  assert.equal(restart.peekLibraryPreview(owner(2)), undefined)
  restart.saveLibraryPreview(owner(2), { summary: summary(2) })
  restart.saveLibraryPreview(owner(3), { summary: summary(3) })
  assert.equal(restart.peekLibraryPreview(owner(1)), undefined)
  assert.ok(restart.peekLibraryPreview(owner(2)))
  assert.ok(restart.peekLibraryPreview(owner(3)))
})
test('mismatched owners, excessive data, corrupt storage and expired previews are rejected', () => {
  const app = harness()
  app.saveLibraryPreview(owner(1), { summary: summary(2) })
  assert.equal(app.peekLibraryPreview(owner(1)), undefined)
  app.saveLibraryPreview(owner(1), { collections: collections('x'.repeat(128 * 1024)) })
  assert.equal(app.peekLibraryPreview(owner(1)), undefined)
  app.saveLibraryPreview(owner(1), { summary: summary(1) })
  app.expire()
  assert.equal(app.peekLibraryPreview(owner(1)), undefined)
  assert.equal(harness('{broken').peekLibraryPreview(owner(1)), undefined)
  app.clearLibraryPreviews()
  assert.equal(app.storage.raw, null)
})
test('storage failures preserve an in-memory preview without blocking the caller', () => {
  const app = harness()
  app.storage.setItem = () => {
    throw Error('quota')
  }
  assert.doesNotThrow(() => app.saveLibraryPreview(owner(1), { summary: summary(1) }))
  assert.equal(app.peekLibraryPreview(owner(1)).summary.profile.userId, '1')
  app.storage.removeItem = () => {
    throw Error('denied')
  }
  assert.doesNotThrow(() => app.clearLibraryPreviews())
  assert.equal(app.peekLibraryPreview(owner(1)), undefined)
})
