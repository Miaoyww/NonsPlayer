import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import { parseQrc } from '@applemusic-like-lyrics/lyric'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/media-lyric-lines.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports }
)
test('AMLL timed lines preserve text and auxiliary lyrics in the native payload', () => {
  const fixture = JSON.parse(readFileSync(new URL('./fixtures/qrc.json', import.meta.url), 'utf8'))
  const lines = parseQrc(fixture.main)
  lines[0].translatedLyric = '  translation  '
  lines[0].romanLyric = ' romanization '
  const value = exports.mediaLyricLines(lines)
  assert.equal(value[0].text, 'Hello world')
  assert.equal(value[0].translation, 'translation')
  assert.equal(value[0].romanization, 'romanization')
  assert.equal(value[0].startTime, 1000)
  assert.equal(value[0].endTime, 2000)
})
test('background lines do not replace the main line; fractional times become native milliseconds', () => {
  const lines = parseQrc('[1000,500]Main(1000,500)\n[1100,500]Background(1100,500)')
  lines[1].isBG = true
  lines[0].startTime += 0.5
  lines[0].endTime += 0.5
  const value = exports.mediaLyricLines(lines)
  assert.equal(value.length, 1)
  assert.equal(value[0].text, 'Main')
  assert.equal(value[0].startTime, 1000)
  assert.equal(value[0].endTime, 1500)
})
