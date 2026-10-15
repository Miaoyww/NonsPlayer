import assert from 'node:assert/strict'
import { test } from 'node:test'
import { sourceCapabilities as capabilities } from './helpers/source-capabilities.mjs'

test('personal page availability is independent from login and audio capabilities', () => {
  const source = (...values) => ({ capabilities: values })
  assert.equal(capabilities.hasPersonalPages(source('account', 'search')), false)
  assert.equal(capabilities.hasPersonalPages(source('search')), false)
  assert.equal(
    capabilities.supportsMusicPage(source('account', 'userLibrary', 'browse'), 'discover'),
    false
  )
  assert.equal(capabilities.supportsMusicPage(source('userLibrary'), 'library'), true)
  assert.equal(capabilities.supportsMusicPage(source('recommendations'), 'discover'), true)
  assert.equal(capabilities.hasPersonalPages(source('recommendations')), true)
})
