import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const lyricCopy = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/lyrics/lyric-copy.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  ).outputText,
  { exports: lyricCopy }
)

function line(text, translatedLyric = '', romanLyric = '') {
  return { words: text ? [{ word: text }] : [], translatedLyric, romanLyric }
}

test('a lyric line copies its main text, translation and romanization in order', () => {
  assert.equal(lyricCopy.lyricLineText(line('Hello', '你好', 'haro')), 'Hello\n你好\nharo')
  assert.equal(lyricCopy.lyricLineText(line('Hello')), 'Hello')
})

test('all lyrics copy every non-empty line and skip interludes', () => {
  assert.equal(
    lyricCopy.lyricsText([line('Hello', '你好'), line(''), line('World', '世界')]),
    'Hello\n你好\nWorld\n世界'
  )
})
