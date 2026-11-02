// Presentation behavior adapted from TaskbarLyrics (MIT). See NOTICE.md.

import type { LyricLine } from '@applemusic-like-lyrics/core'
import { frameAt } from './frame'
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
  incomingTranslationPair: HTMLElement
  incomingTranslationOriginalLine: HTMLElement
  incomingTranslationOriginalText: HTMLElement
  incomingTranslationOriginalScanText: HTMLElement
  incomingTranslationLine: HTMLElement
  incomingTranslationText: HTMLElement
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
    incomingTranslationPair: requiredElement(root, '[data-lyric="incoming-translation-pair"]'),
    incomingTranslationOriginalLine: requiredElement(root, '[data-lyric="incoming-original-line"]'),
    incomingTranslationOriginalText: requiredElement(root, '[data-lyric="incoming-original-text"]'),
    incomingTranslationOriginalScanText: requiredElement(
      root,
      '[data-lyric="incoming-original-scan"]'
    ),
    incomingTranslationLine: requiredElement(root, '[data-lyric="incoming-translation-line"]'),
    incomingTranslationText: requiredElement(root, '[data-lyric="incoming-translation-text"]')
  }
}

function toDisplayLine(line: string) {
  const text = (line ?? '').trim()
  return text.length > 0 ? text : ' '
}

function toTranslationLine(line: string) {
  const text = (line ?? '').trim()
  return text.length > 0 ? text : '…'
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
  private translationMode = false
  private rowHeight = 0
  private linePitch = 0
  private trackBaseY = 0
  private metricsPending = false
  private secondaryOpacity = SECONDARY_BASE_OPACITY
  private transitionStartTime = 0
  private transitionBaseSecondaryOpacity = SECONDARY_BASE_OPACITY
  private transitionPromotedLineIndex = -1
  private transitionUsesTranslationPair = false
  private disposed = false

  constructor(root: HTMLElement) {
    this.elements = readElements(root)
    this.elements.root.classList.add('single-line')
    this.elements.root.classList.remove('translation-mode')
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
    this.refreshHorizontalScroll()
  }

  update(positionMs: number, playing: boolean, animate = true) {
    this.playing = playing
    this.applyFrame(frameAt(this.lines, positionMs, this.trackId, playing), animate)
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
      case TRANSITIONS.TRANSLATION_PAIR_ROLL:
        this.startTranslationPairRoll(target)
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
    this.setTranslationMode(frame.translationMode)
    this.setCurrentLine(frame.current)
    this.setWordScanProgress(frame.wordScanProgress)
    this.setSecondaryLine(frame.translationMode ? frame.currentTranslation : ' ')
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
    this.setTranslationMode(frame.translationMode)
    this.setCurrentLine(frame.current)
    this.setWordScanProgress(frame.wordScanProgress, allowWordScanSmoothing)
    this.setSecondaryLine(frame.translationMode ? frame.currentTranslation : ' ')
    this.updateSecondaryOpacity(frame.progress)
  }

  private startSingleRoll(target: DesktopLyricsFrame) {
    this.cancelActiveTransition()
    this.measure()
    this.setTranslationMode(false)
    this.transitionUsesTranslationPair = false
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
      toDisplayLine(target.current)
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
    this.setCurrentLine(target.current)
    this.setWordScanProgress(target.wordScanProgress, false)
    this.setSecondaryLine(' ')
    this.setLineWordScanProgress(this.elements.secondaryLine, null)
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

  private startTranslationPairRoll(target: DesktopLyricsFrame) {
    this.cancelActiveTransition()
    this.measure()
    this.setTranslationMode(true)
    this.transitionUsesTranslationPair = true
    this.transitionPromotedLineIndex = target.currentLineIndex

    const pair = this.elements.incomingTranslationPair
    const context = this.coordinator.beginTransition(
      {
        kind: TRANSITIONS.TRANSLATION_PAIR_ROLL,
        durationMs: DEFAULT_DURATION_MS.translationPairRoll
      },
      { complete: () => this.finalizeTranslationPairRoll(target) }
    )

    this.elements.track.classList.add('no-anim')
    this.elements.track.classList.remove('animating', 'translation-pair-animating')
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setTrackOffset(0)
    this.clearIncomingTranslationPair()
    pair.classList.add('preparing', 'no-anim')
    this.setIncomingTranslationPair(
      target.current,
      target.currentTranslation,
      target.wordScanProgress
    )
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
      this.elements.track.classList.add('translation-pair-animating', 'animating')
      context.requestFrame(() => {
        if (!context.isCurrent()) return
        this.setTrackOffset(2)
        if (this.prefersReducedMotion()) context.requestFrame(finish)
      })
    })
    context.scheduleFallback(finish, DEFAULT_DURATION_MS.translationPairRoll + 120)
  }

  private finalizeTranslationPairRoll(target: DesktopLyricsFrame) {
    this.elements.track.classList.add('no-anim')
    this.elements.incomingTranslationPair.classList.add('no-anim')
    this.setCurrentLine(target.current)
    this.setWordScanProgress(target.wordScanProgress, false)
    this.setSecondaryLine(target.currentTranslation)
    this.setLineWordScanProgress(this.elements.secondaryLine, null)
    this.elements.track.classList.remove(
      'translation-pair-animating',
      'animating',
      'word-scan-transition'
    )
    this.elements.currentLine.classList.remove('leaving')
    this.elements.secondaryLine.classList.remove('promoting')
    this.setTrackOffset(0)
    this.elements.currentLine.style.opacity = ''
    this.elements.secondaryLine.style.opacity = ''
    this.clearIncomingTranslationPair()
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
    const fadeOut = getFadeOutEase(progress)
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
      'translation-pair-animating',
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
    this.clearIncomingTranslationPair()
    void this.elements.track.offsetHeight
    this.elements.track.classList.remove('no-anim')
    this.transitionUsesTranslationPair = false
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
    const rowCount = this.translationMode ? 2 : 1
    const pairHeight = rowCount * this.rowHeight + Math.max(0, rowCount - 1) * gap
    this.trackBaseY = 0
    this.elements.root.style.setProperty('--desktop-pair-height', `${pairHeight}px`)
    this.elements.root.style.setProperty('--track-base-y', `${this.trackBaseY}px`)
  }

  private setCurrentLine(line: string) {
    this.setLineText(
      this.elements.currentLine,
      this.elements.currentText,
      this.elements.currentScanText,
      toDisplayLine(line)
    )
  }

  private setSecondaryLine(line: string) {
    const text = this.translationMode ? toTranslationLine(line) : ' '
    this.setLineText(
      this.elements.secondaryLine,
      this.elements.secondaryText,
      this.elements.secondaryScanText,
      text
    )
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

  private setTranslationMode(enabled: boolean) {
    if (this.translationMode === enabled) return
    this.translationMode = enabled
    this.elements.root.classList.toggle('translation-mode', enabled)
    this.elements.root.classList.toggle('single-line', !enabled)
    this.elements.secondaryLine.classList.toggle('translation-line', enabled)
    this.updateTrackBase(
      Number.parseFloat(window.getComputedStyle(this.elements.track).rowGap) || 0
    )
    if (enabled) {
      this.setLineWordScanProgress(this.elements.secondaryLine, null)
      this.elements.secondaryLine.style.opacity = ''
    } else {
      this.clearIncomingTranslationPair()
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
    const line = this.transitionUsesTranslationPair
      ? this.elements.incomingTranslationOriginalLine
      : this.elements.secondaryLine
    this.setLineWordScanProgress(line, progress)
  }

  private updateSecondaryOpacity(progress: number) {
    if (this.translationMode) {
      this.elements.secondaryLine.style.opacity = ''
      return
    }
    const target = 0.58 + (1 - clamp01(progress)) * 0.16
    this.secondaryOpacity += (target - this.secondaryOpacity) * 0.28
    this.elements.secondaryLine.style.opacity = this.secondaryOpacity.toFixed(3)
  }

  private setIncomingTranslationPair(
    original: string,
    translation: string,
    wordScanProgress: number | null
  ) {
    this.setLineText(
      this.elements.incomingTranslationOriginalLine,
      this.elements.incomingTranslationOriginalText,
      this.elements.incomingTranslationOriginalScanText,
      toDisplayLine(original)
    )
    this.setLineText(
      this.elements.incomingTranslationLine,
      this.elements.incomingTranslationText,
      null,
      toTranslationLine(translation)
    )
    this.setLineWordScanProgress(
      this.elements.incomingTranslationOriginalLine,
      wordScanProgress,
      false
    )
  }

  private clearIncomingTranslationPair() {
    this.elements.incomingTranslationPair.classList.remove('preparing', 'entering', 'no-anim')
    this.setLineWordScanProgress(this.elements.incomingTranslationOriginalLine, null)
    this.setLineText(
      this.elements.incomingTranslationOriginalLine,
      this.elements.incomingTranslationOriginalText,
      this.elements.incomingTranslationOriginalScanText,
      ' '
    )
    this.setLineText(
      this.elements.incomingTranslationLine,
      this.elements.incomingTranslationText,
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
