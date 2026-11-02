import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function loadModule(relativePath, modules = {}, globals = {}) {
  const exports = {}
  const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8')
  runInNewContext(
    ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2020
      }
    }).outputText,
    {
      exports,
      require: (name) => modules[name],
      ...globals
    }
  )
  return exports
}

function loadDesktopLyrics() {
  const display = loadModule('../src/features/desktop-lyrics/lyric-display.ts')
  const presentation = loadModule('../src/features/desktop-lyrics/presentation.ts', {
    './lyric-display': display
  })
  const frame = loadModule('../src/features/desktop-lyrics/frame.ts', {
    './presentation': presentation,
    './lyric-display': display
  })
  return { display, presentation, frame }
}

function loadStage(globals) {
  const display = loadModule('../src/features/desktop-lyrics/lyric-display.ts', {}, globals)
  const presentation = loadModule(
    '../src/features/desktop-lyrics/presentation.ts',
    { './lyric-display': display },
    globals
  )
  const frame = loadModule('../src/features/desktop-lyrics/frame.ts', {
    './presentation': presentation,
    './lyric-display': display
  })
  const stage = loadModule(
    '../src/features/desktop-lyrics/lyric-stage.ts',
    {
      './presentation': presentation,
      './frame': frame,
      './lyric-display': display
    },
    globals
  )
  return stage
}

function line({ startTime, endTime, words, translatedLyric = '', romanLyric = '' }) {
  return {
    startTime,
    endTime,
    translatedLyric,
    romanLyric,
    words: words.map(([word, wordStart, wordEnd]) => ({
      word,
      startTime: wordStart,
      endTime: wordEnd
    }))
  }
}

test('desktop lyrics line lookup follows the parsed line timeline', () => {
  const { frame } = loadDesktopLyrics()
  const lines = [
    line({ startTime: 1000, endTime: 2000, words: [['第一句', 1000, 2000]] }),
    line({ startTime: 3000, endTime: 4000, words: [['第二句', 3000, 4000]] })
  ]
  assert.equal(frame.lineIndexAt(lines, 0), -1)
  assert.equal(frame.lineIndexAt(lines, 1000), 0)
  assert.equal(frame.lineIndexAt(lines, 2999), 0)
  assert.equal(frame.lineIndexAt(lines, 3000), 1)
  assert.equal(frame.lineIndexAt(lines, 9000), 1)
})

test('desktop lyrics word scan advances across parsed word timings', () => {
  const { frame } = loadDesktopLyrics()
  const current = line({
    startTime: 0,
    endTime: 1000,
    words: [
      ['你', 0, 500],
      ['好', 500, 1000]
    ]
  })

  assert.equal(frame.wordScanProgress(current, -1), 0)
  assert.equal(frame.wordScanProgress(current, 250), 0.25)
  assert.equal(frame.wordScanProgress(current, 750), 0.75)
  assert.equal(frame.wordScanProgress(current, 1000), 1)
})

test('desktop lyrics planner patches progress and rolls only when the line changes', () => {
  const { presentation } = loadDesktopLyrics()
  const planner = new presentation.PresentationPlanner()
  const firstLine = line({
    startTime: 0,
    endTime: 1000,
    words: [['第一句', 0, 1000]],
    translatedLyric: 'First'
  })
  const secondLine = line({
    startTime: 1000,
    endTime: 2000,
    words: [['第二句', 1000, 2000]],
    translatedLyric: 'Second'
  })
  const first = {
    scene: 'lyrics',
    line: firstLine,
    showTranslation: true,
    showPronunciation: true,
    currentLineIndex: 0,
    trackId: 'track'
  }
  const next = {
    ...first,
    line: secondLine,
    currentLineIndex: 1
  }

  assert.equal(planner.plan(first, { ...first, progress: 0.5 }).kind, 'progressPatch')
  assert.equal(
    planner.plan(first, {
      ...next,
      showTranslation: false,
      showPronunciation: false
    }).kind,
    presentation.TRANSITIONS.SINGLE_ROLL
  )
  assert.equal(planner.plan(first, next).kind, presentation.TRANSITIONS.AUXILIARY_ROLL)
  assert.equal(
    planner.plan(first, next, { reducedMotion: true }).kind,
    presentation.TRANSITIONS.IMMEDIATE
  )
})

test('desktop lyrics stage finishes a translation roll with one visible pair', async () => {
  const dom = new JSDOM(`
    <div id="root" class="desktop-lyrics-stage">
      <div class="desktop-lyrics-line auxiliary-line auxiliary-row-measure"></div>
      <div data-lyric="viewport">
        <div data-lyric="track">
          <div data-lyric="current-line">
            <span class="line-text-viewport"><span class="line-text-stack">
              <span class="line-text line-text-base" data-lyric="current-text"></span>
              <span class="line-text line-text-scan" data-lyric="current-scan"></span>
            </span></span>
          </div>
          <div class="secondary-line" data-lyric="secondary-line">
            <span class="line-text-viewport"><span class="line-text-stack">
              <span class="line-text line-text-base" data-lyric="secondary-text"></span>
              <span class="line-text line-text-scan" data-lyric="secondary-scan"></span>
            </span></span>
          </div>
          <div class="tertiary-line" data-lyric="tertiary-line">
            <span class="line-text-viewport"><span class="line-text-stack">
              <span class="line-text line-text-base" data-lyric="tertiary-text"></span>
              <span class="line-text line-text-scan" data-lyric="tertiary-scan"></span>
            </span></span>
          </div>
          <div data-lyric="incoming-auxiliary-pair">
            <div data-lyric="incoming-original-line">
              <span class="line-text-viewport"><span class="line-text-stack">
                <span class="line-text line-text-base" data-lyric="incoming-original-text"></span>
                <span class="line-text line-text-scan" data-lyric="incoming-original-scan"></span>
              </span></span>
            </div>
            <div data-lyric="incoming-auxiliary-1-line">
              <span class="line-text-viewport"><span class="line-text-stack">
                <span class="line-text line-text-base" data-lyric="incoming-auxiliary-1-text"></span>
              </span></span>
            </div>
            <div class="tertiary-line" data-lyric="incoming-auxiliary-2-line">
              <span class="line-text-viewport"><span class="line-text-stack">
                <span class="line-text line-text-base" data-lyric="incoming-auxiliary-2-text"></span>
              </span></span>
            </div>
          </div>
        </div>
      </div>
    </div>
  `)
  const { window } = dom
  const { DesktopLyricStage } = loadStage({ window, document: window.document })
  const root = window.document.getElementById('root')
  const viewport = root.querySelector('[data-lyric="viewport"]')
  const currentLine = root.querySelector('[data-lyric="current-line"]')
  currentLine.getBoundingClientRect = () => ({ height: 40 })
  Object.defineProperty(viewport, 'clientHeight', { value: 80 })
  window.matchMedia = () => ({ matches: false })
  window.requestAnimationFrame = (callback) => setTimeout(() => callback(performance.now()), 0)
  window.cancelAnimationFrame = (id) => clearTimeout(id)
  window.setTimeout = setTimeout
  window.clearTimeout = clearTimeout
  window.devicePixelRatio = 1

  const stage = new DesktopLyricStage(root)
  const first = line({
    startTime: 0,
    endTime: 1000,
    words: [['第一句', 0, 1000]],
    translatedLyric: 'First line'
  })
  const second = line({
    startTime: 1000,
    endTime: 2000,
    words: [['第二句', 1000, 2000]],
    translatedLyric: 'Second line'
  })
  const third = line({
    startTime: 2000,
    endTime: 3000,
    words: [['第三句', 2000, 3000]]
  })
  stage.setSource([first, second, third], 'track')
  stage.update(100, true)
  await new Promise((resolve) => setTimeout(resolve, 1000))

  assert.equal(root.querySelector('[data-lyric="current-text"]').textContent, '第一句')
  assert.equal(root.querySelector('[data-lyric="secondary-text"]').textContent, 'First line')
  assert.equal(
    root.querySelector('[data-lyric="incoming-auxiliary-pair"]').className.includes('entering'),
    false
  )
  assert.equal(root.style.getPropertyValue('--desktop-pair-height'), '80px')

  stage.setDoubleLine(false)
  stage.update(100, true)
  assert.equal(root.querySelector('[data-lyric="secondary-text"]').textContent, ' ')
  assert.equal(root.style.getPropertyValue('--desktop-pair-height'), '40px')

  stage.setDoubleLine(true)
  stage.update(100, true)
  assert.equal(root.querySelector('[data-lyric="secondary-text"]').textContent, 'First line')
  assert.equal(root.style.getPropertyValue('--desktop-pair-height'), '80px')

  stage.update(1100, true)
  await new Promise((resolve) => setTimeout(resolve, 1000))
  assert.equal(root.querySelector('[data-lyric="current-text"]').textContent, '第二句')
  assert.equal(root.querySelector('[data-lyric="secondary-text"]').textContent, 'Second line')
  assert.equal(
    root.querySelector('[data-lyric="incoming-auxiliary-pair"]').className.includes('entering'),
    false
  )

  stage.update(2100, true)
  await new Promise((resolve) => setTimeout(resolve, 1000))
  assert.equal(root.querySelector('[data-lyric="current-text"]').textContent, '第三句')
  assert.equal(root.querySelector('[data-lyric="secondary-text"]').textContent, ' ')
  assert.equal(root.style.getPropertyValue('--desktop-pair-height'), '80px')
  stage.destroy()
  dom.window.close()
})
