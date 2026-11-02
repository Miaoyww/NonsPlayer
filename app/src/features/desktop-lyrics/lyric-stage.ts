// Presentation behavior adapted from TaskbarLyrics (MIT). See NOTICE.md.

import type { LyricLine } from '@applemusic-like-lyrics/core'
import { frameAt } from './frame'
import { auxiliaryLines, lyricText } from './lyric-display'
import {
  DEFAULT_DURATION_MS,
  PresentationCoordinator,
  PresentationPlanner,
  TRANSITIONS,
  clamp01,
  normalizeFrame,
  type DesktopLyricsFrame,
  type PendingFrame
} from './presentation'

const CURRENT_LINE_REST_OPACITY = 0.98
const LEAVING_LINE_OPACITY = 0.16
const SECONDARY_BASE_OPACITY = 0.62
const HORIZONTAL_SCROLL_ANCHOR_RATIO = 0.65
const SINGLE_ROLL_FADE_OUT_MS = 240

interface StageElements {
  root: HTMLElement
  viewport: HTMLElement
  track: HTMLElement
  currentLine: HTMLElement
  currentText: HTMLElement
  currentScanText: HTMLElement
  secondaryLine: HTMLElement
  secondaryText: HTMLElement
  secondaryScanText: HTMLElement
  tertiaryLine: HTMLElement
  tertiaryText: HTMLElement
  tertiaryScanText: HTMLElement
  incomingAuxiliaryPair: HTMLElement
  incomingOriginalLine: HTMLElement
  incomingOriginalText: HTMLElement
  incomingOriginalScanText: HTMLElement
  incomingAuxiliary1Line: HTMLElement
  incomingAuxiliary1Text: HTMLElement
  incomingAuxiliary2Line: HTMLElement
  incomingAuxiliary2Text: HTMLElement
}

interface LineScrollMetrics {
  textStack: HTMLElement
  viewportWidth: number
  contentWidth: number
  overflowWidth: number
}

function requiredElement<T extends HTMLElement>(root: HTMLElement, selector: string) {
  const element = root.querySelector<T>(selector)
  if (!element) throw new Error(`桌面歌词缺少元素：${selector}`)
  return element
}

function readElements(root: HTMLElement): StageElements {
  return {
    root,
    viewport: requiredElement(root, '[data-lyric="viewport"]'),
    track: requiredElement(root, '[data-lyric="track"]'),
    currentLine: requiredElement(root, '[data-lyric="current-line"]'),
    currentText: requiredElement(root, '[data-lyric="current-text"]'),
    currentScanText: requiredElement(root, '[data-lyric="current-scan"]'),
    secondaryLine: requiredElement(root, '[data-lyric="secondary-line"]'),
    secondaryText: requiredElement(root, '[data-lyric="secondary-text"]'),
    secondaryScanText: requiredElement(root, '[data-lyric="secondary-scan"]'),
    tertiaryLine: requiredElement(root, '[data-lyric="tertiary-line"]'),
    tertiaryText: requiredElement(root, '[data-lyric="tertiary-text"]'),
    tertiaryScanText: requiredElement(root, '[data-lyric="tertiary-scan"]'),
    incomingAuxiliaryPair: requiredElement(root, '[data-lyric="incoming-auxiliary-pair"]'),
    incomingOriginalLine: requiredElement(root, '[data-lyric="incoming-original-line"]'),
    incomingOriginalText: requiredElement(root, '[data-lyric="incoming-original-text"]'),
    incomingOriginalScanText: requiredElement(root, '[data-lyric="incoming-original-scan"]'),
    incomingAuxiliary1Line: requiredElement(root, '[data-lyric="incoming-auxiliary-1-line"]'),
    incomingAuxiliary1Text: requiredElement(root, '[data-lyric="incoming-auxiliary-1-text"]'),
    incomingAuxiliary2Line: requiredElement(root, '[data-lyric="incoming-auxiliary-2-line"]'),
    incomingAuxiliary2Text: requiredElement(root, '[data-lyric="incoming-auxiliary-2-text"]')
  }
}

function toDisplayLine(line: string) {
  const text = (line ?? '').trim()
  return text.length > 0 ? text : ' '
}

function toAuxiliaryLine(line: string) {
  const text = (line ?? '').trim()
  return text.length > 0 ? text : '…'
}

function frameAuxiliaryLines(frame: DesktopLyricsFrame) {
  return auxiliaryLines(frame.line, {
    translation: frame.showTranslation,
    pronunciation: frame.showPronunciation
  })
}

function easeOutCubic(value: number) {
  const x = 1 - clamp01(value)
  return 1 - x * x * x
}

function getFadeOutEase(value: number) {
  const normalized = clamp01(value / 0.74)
  return normalized >= 0.97 ? 1 : easeOutCubic(normalized)
}

function getFadeInEase(value: number) {
  const normalized = clamp01(value / 0.72)
  return normalized >= 0.96 ? 1 : easeOutCubic(normalized)
}

export class DesktopLyricStage {
  private readonly elements: StageElements
  private readonly planner = new PresentationPlanner()
  private readonly coordinator = new PresentationCoordinator(this.planner)
  private readonly reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  private readonly horizontalScrollMetrics = new WeakMap<HTMLElement, LineScrollMetrics>()
  private readonly resizeObserver: ResizeObserver | undefined
  private lines: readonly LyricLine[] = []
  private trackId = ''
  private playing = false
  private auxiliaryCount = 0
  private reservedAuxiliaryCount = 0
  private auxiliarySignature = ''
  private showTranslation = true
  private showPronunciation = true
  private rowHeight = 0
  private linePitch = 0
  private trackBaseY = 0
  private metricsPending = false
  private secondaryOpacity = SECONDARY_BASE_OPACITY
  private transitionStartTime = 0
  private transitionBaseSecondaryOpacity = SECONDARY_BASE_OPACITY
  private transitionPromotedLineIndex = -1
  private transitionUsesAuxiliaryPair = false
  private disposed = false

  constructor(root: HTMLElement) {
    this.elements = readElements(root)
    this.elements.root.classList.add('single-line')
    this.elements.root.classList.remove('auxiliary-mode', 'double-auxiliary')
    this.measure()
    if (typeof ResizeObserver !== 'undefined') {
      this.resizeObserver = new ResizeObserver(() => {
        this.measure()
        this.refreshHorizontalScroll()
      })
      this.resizeObserver.observe(this.elements.root)
      this.resizeObserver.observe(this.elements.viewport)
    }
    if (document.fonts?.ready) {
      void document.fonts.ready
        .then(() => {
          if (this.disposed) return
          this.measure()
          this.refreshHorizontalScroll()
        })
        .catch(() => {})
    }
  }

  setSource(lines: readonly LyricLine[], trackId: string) {
    this.lines = lines
    this.trackId = trackId
    this.refreshReservedAuxiliaryCount()
    this.refreshHorizontalScroll()
  }

  setAuxiliaryVisibility(showTranslation: boolean, showPronunciation: boolean) {
    this.showTranslation = showTranslation
    this.showPronunciation = showPronunciation
    this.refreshReservedAuxiliaryCount()
  }

  update(positionMs: number, playing: boolean, animate = true) {
    this.playing = playing
    this.applyFrame(
      frameAt(this.lines, positionMs, this.trackId, playing, {
        translation: this.showTranslation,
        pronunciation: this.showPronunciation
      }),
      animate
    )
  }

  refresh() {
    this.measure()
    this.refreshHorizontalScroll()
  }

  destroy() {
    this.disposed = true
    this.coordinator.cancelTransition()
    this.resizeObserver?.disconnect()
  }

  private applyFrame(targetFrame: DesktopLyricsFrame, animate: boolean) {
    const target = this.stabilizeFrame(normalizeFrame(targetFrame))
    if (!target) return
    if (this.coordinator.isTransitioning) {
      this.coordinator.queueLatest(target, { animate })
      this.updateTransitionWordScan(target.wordScanProgress)
      return
    }

    const plan = this.planner.plan(this.coordinator.currentFrame, target, {
      animateTransition: animate,
      reducedMotion: this.prefersReducedMotion()
    })

    switch (plan.kind) {
      case TRANSITIONS.PROGRESS_PATCH:
        this.applyProgressPatch(target)
        return
      case TRANSITIONS.REPLACE_IN_PLACE:
      case TRANSITIONS.IMMEDIATE:
        this.applyWithoutTransition(target)
        return
      case TRANSITIONS.SINGLE_ROLL:
        this.startSingleRoll(target)
        return
      case TRANSITIONS.AUXILIARY_ROLL:
        this.startAuxiliaryRoll(target)
    }
  }

  private stabilizeFrame(target: DesktopLyricsFrame) {
    const current = this.coordinator.currentFrame
    if (
      !this.playing ||
      current.scene !== 'lyrics' ||
      target.scene !== 'lyrics' ||
      current.trackId !== target.trackId
    ) {
      return target
    }

    if (
      target.currentLineIndex < current.currentLineIndex &&
      target.currentLineIndex >= current.currentLineIndex - 1
    ) {
      return null
    }

    if (
      target.currentLineIndex === current.currentLineIndex &&
      target.wordScanProgress !== null &&
      current.wordScanProgress !== null &&
      target.wordScanProgress < current.wordScanProgress
    ) {
      const difference = current.wordScanProgress - target.wordScanProgress
      if (difference <= 0.15) return current
    }

    return target
  }

  private applyProgressPatch(frame: DesktopLyricsFrame) {
    this.setAuxiliaryLines(frameAuxiliaryLines(frame))
    this.setCurrentLine(lyricText(frame.line))
    this.setWordScanProgress(frame.wordScanProgress)
    this.updateSecondaryOpacity(frame.progress)
    this.coordinator.setCurrentFrame(frame)
  }

  private applyWithoutTransition(frame: DesktopLyricsFrame) {
    this.cancelActiveTransition()
    this.renderFrameContent(frame, false)
    this.coordinator.setCurrentFrame(frame)
    this.consumePendingFrame()
  }

  private renderFrameContent(frame: DesktopLyricsFrame, allowWordScanSmoothing: boolean) {
    this.setAuxiliaryLines(frameAuxiliaryLines(frame))
    this.setCurrentLine(lyricText(frame.line))
    this.setWordScanProgress(frame.wordScanProgress, allowWordScanSmoothing)
    this.updateSecondaryOpacity(frame.progress)
  }

  private startSingleRoll(target: DesktopLyricsFrame) {
    this.cancelActiveTransition()
    this.measure()
    this.setAuxiliaryCount(0)
    this.transitionUsesAuxiliaryPair = false
    this.transitionPromotedLineIndex = target.currentLineIndex
    this.elements.track.classList.add('single-line-roll')

    const currentFontSize = Number.parseFloat(
      window.getComputedStyle(this.elements.currentLine).fontSize
    )
    const secondaryFontSize = Number.parseFloat(
      window.getComputedStyle(this.elements.secondaryLine).fontSize
    )
    const promotionStartScale =
      currentFontSize > 0 && secondaryFontSize > 0
        ? clamp01(secondaryFontSize / currentFontSize)
        : 1
    this.transitionBaseSecondaryOpacity = this.secondaryOpacity

    const context = this.coordinator.beginTransition(
      { kind: TRANSITIONS.SINGLE_ROLL, durationMs: DEFAULT_DURATION_MS.singleRoll },
      { complete: () => this.finalizeSingleRoll(target) }
    )

    this.elements.track.classList.add('no-anim')
    this.elements.track.classList.remove('animating')
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setTrackOffset(0)
    this.setLineText(
      this.elements.secondaryLine,
      this.elements.secondaryText,
      this.elements.secondaryScanText,
      toDisplayLine(lyricText(target.line))
    )
    this.elements.currentLine.style.opacity = String(CURRENT_LINE_REST_OPACITY)
    this.elements.secondaryLine.style.opacity = String(this.transitionBaseSecondaryOpacity)
    if (currentFontSize > 0) {
      this.elements.secondaryLine.style.fontSize = `${currentFontSize.toFixed(3)}px`
    }
    this.elements.secondaryLine.style.setProperty(
      '--promotion-scale',
      promotionStartScale.toFixed(6)
    )
    this.elements.secondaryLine.style.transform = 'translateY(0) scale(var(--promotion-scale))'
    this.setLineWordScanProgress(this.elements.secondaryLine, target.wordScanProgress, false)
    void this.elements.track.offsetHeight
    this.elements.track.classList.remove('no-anim')

    let finished = false
    const finish = () => {
      if (finished) return
      finished = true
      context.finish()
    }
    context.listenTransitionEnd(this.elements.track, (event) => {
      if (
        event.target !== this.elements.track ||
        (event as TransitionEvent).propertyName !== 'transform'
      ) {
        return
      }
      finish()
    })
    context.requestFrame(() => {
      this.transitionStartTime = window.performance.now()
      context.requestFrame(this.runSingleRollOpacity)
      this.elements.currentLine.classList.add('leaving')
      this.elements.secondaryLine.classList.add('promoting')
      this.elements.secondaryLine.style.setProperty('--promotion-scale', '1')
      this.elements.secondaryLine.style.removeProperty('transform')
      this.elements.track.classList.add('animating')
      this.setTrackOffset(1)
    })
    context.scheduleFallback(finish, DEFAULT_DURATION_MS.singleRoll + 120)
  }

  private finalizeSingleRoll(target: DesktopLyricsFrame) {
    this.elements.track.classList.add('no-anim')
    this.elements.currentLine.style.opacity = String(LEAVING_LINE_OPACITY)
    this.elements.secondaryLine.style.opacity = String(CURRENT_LINE_REST_OPACITY)
    void this.elements.track.offsetHeight
    this.setCurrentLine(lyricText(target.line))
    this.setWordScanProgress(target.wordScanProgress, false)
    this.clearAuxiliaryLines()
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.elements.track.classList.remove('animating', 'single-line-roll')
    this.setTrackOffset(0)
    this.elements.currentLine.style.opacity = ''
    this.elements.secondaryLine.style.opacity = ''
    this.elements.secondaryLine.style.fontSize = ''
    this.elements.secondaryLine.style.transform = ''
    this.elements.secondaryLine.style.removeProperty('--promotion-scale')
    void this.elements.track.offsetHeight
    this.elements.track.classList.remove('no-anim')
    this.updateSecondaryOpacity(target.progress)
    this.finishFrame(target)
  }

  private startAuxiliaryRoll(target: DesktopLyricsFrame) {
    this.cancelActiveTransition()
    this.measure()
    const auxiliary = frameAuxiliaryLines(target)
    this.setAuxiliaryCount(auxiliary.length)
    this.transitionUsesAuxiliaryPair = true
    this.transitionPromotedLineIndex = target.currentLineIndex

    const pair = this.elements.incomingAuxiliaryPair
    const context = this.coordinator.beginTransition(
      {
        kind: TRANSITIONS.AUXILIARY_ROLL,
        durationMs: DEFAULT_DURATION_MS.auxiliaryRoll
      },
      { complete: () => this.finalizeAuxiliaryRoll(target) }
    )

    this.elements.track.classList.add('no-anim')
    this.elements.track.classList.remove('animating', 'auxiliary-pair-animating')
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setTrackOffset(0)
    this.clearIncomingAuxiliaryPair()
    pair.classList.add('preparing', 'no-anim')
    this.setIncomingAuxiliaryPair(lyricText(target.line), auxiliary, target.wordScanProgress)
    this.elements.currentLine.style.opacity = ''
    this.elements.secondaryLine.style.opacity = ''
    void pair.offsetHeight
    this.elements.track.classList.remove('no-anim')
    pair.classList.remove('no-anim')

    let finished = false
    const finish = () => {
      if (finished) return
      finished = true
      context.finish()
    }
    context.listenTransitionEnd(this.elements.track, (event) => {
      if (
        event.target !== this.elements.track ||
        (event as TransitionEvent).propertyName !== 'transform'
      ) {
        return
      }
      finish()
    })
    context.requestFrame(() => {
      pair.classList.remove('preparing')
      pair.classList.add('entering')
      this.elements.track.classList.add('auxiliary-pair-animating', 'animating')
      context.requestFrame(() => {
        if (!context.isCurrent()) return
        this.setTrackOffset(1 + auxiliary.length)
        if (this.prefersReducedMotion()) context.requestFrame(finish)
      })
    })
    context.scheduleFallback(finish, DEFAULT_DURATION_MS.auxiliaryRoll + 120)
  }

  private finalizeAuxiliaryRoll(target: DesktopLyricsFrame) {
    this.elements.track.classList.add('no-anim')
    this.elements.incomingAuxiliaryPair.classList.add('no-anim')
    this.setCurrentLine(lyricText(target.line))
    this.setWordScanProgress(target.wordScanProgress, false)
    this.setAuxiliaryLines(frameAuxiliaryLines(target))
    this.elements.track.classList.remove(
      'auxiliary-pair-animating',
      'animating',
      'word-scan-transition'
    )
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setTrackOffset(0)
    this.elements.currentLine.style.opacity = ''
    this.elements.secondaryLine.style.opacity = ''
    this.clearIncomingAuxiliaryPair()
    void this.elements.track.offsetHeight
    this.elements.track.classList.remove('no-anim')
    this.finishFrame(target)
  }

  private finishFrame(target: DesktopLyricsFrame) {
    this.coordinator.setCurrentFrame(target)
    if (this.metricsPending) this.measure()
    this.consumePendingFrame()
  }

  private consumePendingFrame() {
    const pending = this.coordinator.takeLatest()
    if (pending) this.applyPendingFrame(pending)
  }

  private applyPendingFrame(pending: PendingFrame) {
    this.applyFrame(pending.frame, pending.options.animate !== false)
  }

  private runSingleRollOpacity = (now: number) => {
    if (!this.coordinator.isTransitioning) return
    const elapsed = Math.max(0, now - this.transitionStartTime)
    const progress = clamp01(elapsed / DEFAULT_DURATION_MS.singleRoll)
    const fadeOut = getFadeOutEase(elapsed / SINGLE_ROLL_FADE_OUT_MS)
    const fadeIn = getFadeInEase(progress)
    this.elements.currentLine.style.opacity = String(
      CURRENT_LINE_REST_OPACITY + (LEAVING_LINE_OPACITY - CURRENT_LINE_REST_OPACITY) * fadeOut
    )
    this.elements.secondaryLine.style.opacity = String(
      this.transitionBaseSecondaryOpacity +
        (CURRENT_LINE_REST_OPACITY - this.transitionBaseSecondaryOpacity) * fadeIn
    )
    if (progress < 1) {
      this.coordinator.requestFrame(this.coordinator.activeGeneration, this.runSingleRollOpacity)
    }
  }

  private cancelActiveTransition() {
    this.coordinator.cancelTransition()
    this.coordinator.takeLatest()
    this.elements.track.classList.add('no-anim')
    this.elements.track.classList.remove(
      'animating',
      'single-line-roll',
      'auxiliary-pair-animating',
      'word-scan-transition'
    )
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setLineWordScanProgress(this.elements.secondaryLine, null)
    this.setTrackOffset(0)
    this.elements.currentLine.style.opacity = ''
    this.elements.secondaryLine.style.opacity = ''
    this.elements.secondaryLine.style.fontSize = ''
    this.elements.secondaryLine.style.transform = ''
    this.elements.secondaryLine.style.removeProperty('--promotion-scale')
    this.clearIncomingAuxiliaryPair()
    void this.elements.track.offsetHeight
    this.elements.track.classList.remove('no-anim')
    this.transitionUsesAuxiliaryPair = false
    this.transitionPromotedLineIndex = -1
  }

  private measure() {
    if (this.coordinator.isTransitioning) {
      this.metricsPending = true
      return
    }

    this.metricsPending = false
    const height = this.elements.currentLine.getBoundingClientRect().height
    const gap = Number.parseFloat(window.getComputedStyle(this.elements.track).rowGap) || 0
    if (height > 0) {
      this.rowHeight = height
      this.linePitch = height + gap
      this.elements.root.style.setProperty('--line-pitch', `${this.linePitch}px`)
    }
    this.updateTrackBase(gap)
    this.refreshHorizontalScroll()
  }

  private setTrackOffset(rows: number) {
    if (!this.linePitch) this.measure()
    const distance = -this.linePitch * rows
    const ratio = Number(window.devicePixelRatio) || 1
    const snapped = Math.round((this.trackBaseY + distance) * ratio) / ratio
    this.elements.track.style.transform = `translateY(${snapped}px)`
  }

  private updateTrackBase(gap = 0) {
    const rowCount = 1 + Math.max(this.auxiliaryCount, this.reservedAuxiliaryCount)
    const pairHeight = rowCount * this.rowHeight + Math.max(0, rowCount - 1) * gap
    this.trackBaseY = 0
    this.elements.root.style.setProperty('--desktop-pair-height', `${pairHeight}px`)
    this.elements.root.style.setProperty('--track-base-y', `${this.trackBaseY}px`)
  }

  private refreshReservedAuxiliaryCount() {
    let reserved = 0
    for (const line of this.lines) {
      reserved = Math.max(
        reserved,
        auxiliaryLines(line, {
          translation: this.showTranslation,
          pronunciation: this.showPronunciation
        }).length
      )
      if (reserved === 2) break
    }
    if (reserved === this.reservedAuxiliaryCount) return
    this.reservedAuxiliaryCount = reserved
    this.updateTrackBase(
      Number.parseFloat(window.getComputedStyle(this.elements.track).rowGap) || 0
    )
  }

  private setCurrentLine(line: string) {
    this.setLineText(
      this.elements.currentLine,
      this.elements.currentText,
      this.elements.currentScanText,
      toDisplayLine(line)
    )
  }

  private setAuxiliaryLines(lines: readonly string[]) {
    const visible = lines
      .map((line) => line.trim())
      .filter((line) => line.length > 0)
      .slice(0, 2)
    const signature = visible.join('\n')
    if (signature === this.auxiliarySignature) return
    this.auxiliarySignature = signature
    this.setAuxiliaryCount(visible.length)
    this.setLineText(
      this.elements.secondaryLine,
      this.elements.secondaryText,
      this.elements.secondaryScanText,
      visible[0] ? toAuxiliaryLine(visible[0]) : ' '
    )
    this.setLineText(
      this.elements.tertiaryLine,
      this.elements.tertiaryText,
      this.elements.tertiaryScanText,
      visible[1] ? toAuxiliaryLine(visible[1]) : ' '
    )
    this.setLineWordScanProgress(this.elements.secondaryLine, null)
    this.setLineWordScanProgress(this.elements.tertiaryLine, null)
  }

  private clearAuxiliaryLines() {
    this.auxiliarySignature = ''
    this.setAuxiliaryCount(0)
    this.setLineText(
      this.elements.secondaryLine,
      this.elements.secondaryText,
      this.elements.secondaryScanText,
      ' '
    )
    this.setLineText(
      this.elements.tertiaryLine,
      this.elements.tertiaryText,
      this.elements.tertiaryScanText,
      ' '
    )
    this.setLineWordScanProgress(this.elements.secondaryLine, null)
    this.setLineWordScanProgress(this.elements.tertiaryLine, null)
  }

  private setLineText(
    lineElement: HTMLElement,
    textElement: HTMLElement,
    scanTextElement: HTMLElement | null,
    text: string
  ) {
    if (
      textElement.textContent === text &&
      (!scanTextElement || scanTextElement.textContent === text)
    ) {
      return
    }
    this.clearLineHorizontalScroll(lineElement)
    textElement.textContent = text
    if (scanTextElement) scanTextElement.textContent = text
  }

  private setAuxiliaryCount(count: number) {
    const next = Math.max(0, Math.min(2, count))
    if (this.auxiliaryCount === next) return
    this.auxiliaryCount = next
    this.elements.root.classList.toggle('auxiliary-mode', next > 0)
    this.elements.root.classList.toggle('single-line', next === 0)
    this.elements.root.classList.toggle('double-auxiliary', next === 2)
    this.elements.secondaryLine.classList.toggle('auxiliary-line', next > 0)
    this.elements.tertiaryLine.classList.toggle('auxiliary-line', next === 2)
    this.updateTrackBase(
      Number.parseFloat(window.getComputedStyle(this.elements.track).rowGap) || 0
    )
    if (next > 0) {
      this.setLineWordScanProgress(this.elements.secondaryLine, null)
      this.elements.secondaryLine.style.opacity = ''
    } else {
      this.clearIncomingAuxiliaryPair()
    }
    if (next < 2) {
      this.setLineWordScanProgress(this.elements.tertiaryLine, null)
      this.setLineText(
        this.elements.tertiaryLine,
        this.elements.tertiaryText,
        this.elements.tertiaryScanText,
        ' '
      )
    }
  }

  private setWordScanProgress(progress: number | null, allowSmoothing = true) {
    this.setLineWordScanProgress(this.elements.currentLine, progress, allowSmoothing)
  }

  private setLineWordScanProgress(
    lineElement: HTMLElement,
    progress: number | null,
    allowSmoothing = true
  ) {
    const normalized = progress === null ? null : clamp01(progress)
    const previous = Number.parseFloat(lineElement.style.getPropertyValue('--word-scan-progress'))
    const previousProgress = Number.isFinite(previous) ? previous / 100 : Number.NaN
    const continuous =
      normalized !== null &&
      Number.isFinite(previousProgress) &&
      normalized >= previousProgress - 0.02 &&
      normalized - previousProgress <= 0.35
    lineElement.classList.toggle('word-scanning', normalized !== null)
    lineElement.classList.toggle(
      'word-scan-smoothing',
      normalized !== null &&
        normalized > 0 &&
        normalized < 1 &&
        this.playing &&
        !this.prefersReducedMotion() &&
        allowSmoothing &&
        continuous
    )
    if (normalized === null) {
      lineElement.style.removeProperty('--word-scan-progress')
      this.updateLineHorizontalScroll(lineElement, null)
      return
    }
    lineElement.style.setProperty('--word-scan-progress', `${(normalized * 100).toFixed(3)}%`)
    this.updateLineHorizontalScroll(lineElement, normalized)
  }

  private updateTransitionWordScan(progress: number | null) {
    if (!this.coordinator.isTransitioning || this.transitionPromotedLineIndex < 0) return
    const line = this.transitionUsesAuxiliaryPair
      ? this.elements.incomingOriginalLine
      : this.elements.secondaryLine
    this.setLineWordScanProgress(line, progress)
  }

  private updateSecondaryOpacity(progress: number) {
    if (this.auxiliaryCount > 0) {
      this.elements.secondaryLine.style.opacity = ''
      return
    }
    const target = 0.58 + (1 - clamp01(progress)) * 0.16
    this.secondaryOpacity += (target - this.secondaryOpacity) * 0.28
    this.elements.secondaryLine.style.opacity = this.secondaryOpacity.toFixed(3)
  }

  private setIncomingAuxiliaryPair(
    original: string,
    auxiliary: readonly string[],
    wordScanProgress: number | null
  ) {
    this.setLineText(
      this.elements.incomingOriginalLine,
      this.elements.incomingOriginalText,
      this.elements.incomingOriginalScanText,
      toDisplayLine(original)
    )
    this.setLineText(
      this.elements.incomingAuxiliary1Line,
      this.elements.incomingAuxiliary1Text,
      null,
      auxiliary[0] ? toAuxiliaryLine(auxiliary[0]) : ' '
    )
    this.setLineText(
      this.elements.incomingAuxiliary2Line,
      this.elements.incomingAuxiliary2Text,
      null,
      auxiliary[1] ? toAuxiliaryLine(auxiliary[1]) : ' '
    )
    this.setLineWordScanProgress(this.elements.incomingOriginalLine, wordScanProgress, false)
  }

  private clearIncomingAuxiliaryPair() {
    this.elements.incomingAuxiliaryPair.classList.remove('preparing', 'entering', 'no-anim')
    this.setLineWordScanProgress(this.elements.incomingOriginalLine, null)
    this.setLineText(
      this.elements.incomingOriginalLine,
      this.elements.incomingOriginalText,
      this.elements.incomingOriginalScanText,
      ' '
    )
    this.setLineText(
      this.elements.incomingAuxiliary1Line,
      this.elements.incomingAuxiliary1Text,
      null,
      ' '
    )
    this.setLineText(
      this.elements.incomingAuxiliary2Line,
      this.elements.incomingAuxiliary2Text,
      null,
      ' '
    )
  }

  private refreshHorizontalScroll() {
    this.refreshLineHorizontalScroll(this.elements.currentLine)
  }

  private refreshLineHorizontalScroll(lineElement: HTMLElement) {
    this.horizontalScrollMetrics.delete(lineElement)
    const progress = Number.parseFloat(lineElement.style.getPropertyValue('--word-scan-progress'))
    this.updateLineHorizontalScroll(
      lineElement,
      Number.isFinite(progress) ? clamp01(progress / 100) : null
    )
  }

  private updateLineHorizontalScroll(lineElement: HTMLElement, progress: number | null) {
    if (progress === null) {
      this.clearLineHorizontalScroll(lineElement)
      return
    }
    const metrics = this.measureLineHorizontalScroll(lineElement)
    if (!metrics || metrics.viewportWidth <= 0 || metrics.overflowWidth < 0.5) {
      this.clearLineHorizontalScroll(lineElement, false)
      return
    }
    const scanHead = progress * metrics.contentWidth
    const anchor = metrics.viewportWidth * HORIZONTAL_SCROLL_ANCHOR_RATIO
    const rawOffset = Math.max(0, Math.min(metrics.overflowWidth, scanHead - anchor))
    const ratio = Number(window.devicePixelRatio) || 1
    const offset = Math.round(rawOffset * ratio) / ratio
    lineElement.classList.add('horizontal-scrolling')
    metrics.textStack.style.setProperty('--line-scroll-offset', `${offset === 0 ? 0 : -offset}px`)
  }

  private measureLineHorizontalScroll(lineElement: HTMLElement) {
    const cached = this.horizontalScrollMetrics.get(lineElement)
    if (cached) return cached
    const textViewport = lineElement.querySelector<HTMLElement>('.line-text-viewport')
    const textStack = lineElement.querySelector<HTMLElement>('.line-text-stack')
    const baseText = lineElement.querySelector<HTMLElement>('.line-text-base')
    if (!textViewport || !textStack || !baseText) return null
    const viewportWidth = textViewport.clientWidth
    const contentWidth = baseText.scrollWidth
    const metrics = {
      textStack,
      viewportWidth,
      contentWidth,
      overflowWidth: Math.max(0, contentWidth - viewportWidth)
    }
    if (viewportWidth > 0 && contentWidth > 0) {
      this.horizontalScrollMetrics.set(lineElement, metrics)
    }
    return metrics
  }

  private clearLineHorizontalScroll(lineElement: HTMLElement, discardMetrics = true) {
    if (discardMetrics) this.horizontalScrollMetrics.delete(lineElement)
    lineElement.classList.remove('horizontal-scrolling')
    lineElement
      .querySelector<HTMLElement>('.line-text-stack')
      ?.style.removeProperty('--line-scroll-offset')
  }

  private prefersReducedMotion() {
    return this.reducedMotionQuery.matches
  }
}
