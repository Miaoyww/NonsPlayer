import assert from 'node:assert/strict'
import { test } from 'node:test'
import { updateCargoLock, updateCargoToml } from '../../scripts/release-version.mjs'

test('release version updates are idempotent when Cargo files already match', () => {
  const manifest =
    '[package]\r\nname = "nons"\r\ndefault-run = "Nons"\r\nversion = "1.12.0"\r\n\r\n[lib]\r\nname = "nons_lib"\r\n'
  assert.equal(updateCargoToml(manifest, '1.12.0'), manifest)
  assert.match(updateCargoToml(manifest, '1.12.1'), /^version = "1\.12\.1"$/m)

  const lock = '[[package]]\r\nname = "nons"\r\nversion = "1.12.0"\r\ndependencies = [\r\n]\r\n'
  assert.equal(updateCargoLock(lock, '1.12.0'), lock)
  assert.match(updateCargoLock(lock, '1.12.1'), /^version = "1\.12\.1"$/m)
})

test('release version rejects a Cargo manifest without a package version', () => {
  assert.throws(() => updateCargoToml('[package]\nname = "nons"\n', '1.12.0'), /缺少 version/)
})
