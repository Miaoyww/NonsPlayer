import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function loadDiagnostics(reports) {
  const exports = {}
  const source = readFileSync(
    new URL('../src/features/lyrics/current-lyric-diagnostics.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }
    }).outputText,
    {
      exports,
      console: { info() {} },
      require: () => ({ reportDiagnostic: (entry) => reports.push(entry) })
    }
  )
  return exports
}

function line(startTime, endTime, words) {
  return {
    startTime,
    endTime,
    translatedLyric: '',
    romanLyric: '',
    words: words.map(([word, wordStart, wordEnd]) => ({
      word,
      startTime: wordStart,
      endTime: wordEnd
    }))
  }
}

test('current lyric diagnostics include the current line and adjacent word timings', () => {
  const reports = []
  const diagnostics = loadDiagnostics(reports)
  const lines = [
    line(1000, 2000, [['前', 1000, 2000]]),
    line(2000, 3000, [
      ['当', 2000, 2400],
      ['前', 2400, 3000]
    ]),
    line(3000, 4000, [['后', 3000, 4000]])
  ]
  const input = {
    operation: 'lyrics.test',
    trackKey: '{"source":"netease"}',
    trackLabel: 'Artist / Title',
    positionMs: 2600,
    lyrics: { source: 'qq', format: 'qrc', content: '', translation: null, romanization: null },
    lines
  }
  const message = diagnostics.formatCurrentLyricDiagnostic(input)

  assert.match(message, /currentLine=1/)
  assert.match(message, /line=0;range=1000-2000/)
  assert.match(message, /line=1;range=2000-3000;text=当前;words=当@2000-2400\|前@2400-3000/)
  assert.match(message, /line=2;range=3000-4000/)
  assert.doesNotMatch(message, /[{}\[\]]/)

  diagnostics.reportCurrentLyrics(input)
  assert.equal(reports.length, 1)
  assert.equal(reports[0].command, 'lyrics_data')
  assert.equal(reports[0].source, 'qq')
  assert.equal(reports[0].stage, 'qrc')
  assert.equal(reports[0].message, message)
})
