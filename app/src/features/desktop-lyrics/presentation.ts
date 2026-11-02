// Presentation behavior adapted from TaskbarLyrics (MIT). See NOTICE.md.

export type DesktopLyricsScene = 'lyrics' | 'message'
export type DesktopLyricsLayout = 'single' | 'translationPair'

export interface DesktopLyricsFrame {
  scene: DesktopLyricsScene
  layout: DesktopLyricsLayout
  translationMode: boolean
  current: string
  currentTranslation: string
  currentLineIndex: number
  progress: number
  wordScanProgress: number | null
  trackId: string
  isPlaying: boolean
}

export const TRANSITIONS = {
  PROGRESS_PATCH: 'progressPatch',
  REPLACE_IN_PLACE: 'replaceInPlace',
  SINGLE_ROLL: 'singleRoll',
  TRANSLATION_PAIR_ROLL: 'translationPairRoll',
  IMMEDIATE: 'immediate'
} as const

export type TransitionKind = (typeof TRANSITIONS)[keyof typeof TRANSITIONS]

export interface TransitionPlan {
  kind: TransitionKind
  from: DesktopLyricsFrame
  to: DesktopLyricsFrame
  durationMs: number
}

export const DEFAULT_DURATION_MS = Object.freeze({
  singleRoll: 560,
  translationPairRoll: 760
})

export function clamp01(value: unknown) {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return 0
  return Math.max(0, Math.min(1, parsed))
}

function normalizeOptionalProgress(value: unknown) {
  if (value === null || value === undefined || value === '') return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? clamp01(parsed) : null
}

export function normalizeFrame(frame: Partial<DesktopLyricsFrame> = {}): DesktopLyricsFrame {
  const currentLineIndex = Number(frame.currentLineIndex)
  const layout: DesktopLyricsLayout =
    frame.layout === 'translationPair' || frame.translationMode === true
      ? 'translationPair'
      : 'single'
  return {
    scene: frame.scene === 'lyrics' ? 'lyrics' : 'message',
    layout,
    translationMode: layout === 'translationPair',
    current: String(frame.current ?? ''),
    currentTranslation: String(frame.currentTranslation ?? ''),
    currentLineIndex: Number.isInteger(currentLineIndex) ? currentLineIndex : -1,
    progress: clamp01(frame.progress),
    wordScanProgress: normalizeOptionalProgress(frame.wordScanProgress),
    trackId: String(frame.trackId ?? ''),
    isPlaying: frame.isPlaying !== false
  }
}

function sameLyricsIdentity(current: DesktopLyricsFrame, target: DesktopLyricsFrame) {
  return (
    current.scene === 'lyrics' &&
    target.scene === 'lyrics' &&
    current.currentLineIndex >= 0 &&
    current.currentLineIndex === target.currentLineIndex &&
    current.trackId === target.trackId
  )
}

function makePlan(
  kind: TransitionKind,
  from: DesktopLyricsFrame,
  to: DesktopLyricsFrame,
  durationMs: number
): TransitionPlan {
  return { kind, from, to, durationMs }
}

export class PresentationPlanner {
  plan(
    currentFrame: Partial<DesktopLyricsFrame>,
    targetFrame: Partial<DesktopLyricsFrame>,
    options: { forceImmediate?: boolean; animateTransition?: boolean; reducedMotion?: boolean } = {}
  ) {
    const current = normalizeFrame(currentFrame)
    const target = normalizeFrame(targetFrame)
    const immediate =
      options.forceImmediate === true ||
      options.animateTransition === false ||
      options.reducedMotion === true

    if (immediate) {
      return makePlan(TRANSITIONS.IMMEDIATE, current, target, 0)
    }

    if (sameLyricsIdentity(current, target)) {
      const textChanged =
        current.current !== target.current ||
        current.currentTranslation !== target.currentTranslation ||
        current.layout !== target.layout
      return makePlan(
        textChanged ? TRANSITIONS.REPLACE_IN_PLACE : TRANSITIONS.PROGRESS_PATCH,
        current,
        target,
        0
      )
    }

    if (target.scene === 'lyrics' && current.scene === 'lyrics') {
      const isTranslationPair = target.layout === 'translationPair'
      return makePlan(
        isTranslationPair ? TRANSITIONS.TRANSLATION_PAIR_ROLL : TRANSITIONS.SINGLE_ROLL,
        current,
        target,
        isTranslationPair ? DEFAULT_DURATION_MS.translationPairRoll : DEFAULT_DURATION_MS.singleRoll
      )
    }

    if (target.scene === 'lyrics') {
      const isTranslationPair = target.layout === 'translationPair'
      return makePlan(
        isTranslationPair ? TRANSITIONS.TRANSLATION_PAIR_ROLL : TRANSITIONS.SINGLE_ROLL,
        current,
        target,
        isTranslationPair ? DEFAULT_DURATION_MS.translationPairRoll : DEFAULT_DURATION_MS.singleRoll
      )
    }

    const textChanged =
      current.scene !== target.scene ||
      current.current !== target.current ||
      current.currentTranslation !== target.currentTranslation ||
      current.layout !== target.layout
    return makePlan(
      textChanged ? TRANSITIONS.SINGLE_ROLL : TRANSITIONS.PROGRESS_PATCH,
      current,
      target,
      textChanged ? DEFAULT_DURATION_MS.singleRoll : 0
    )
  }
}

export interface PresentationCallbacks {
  start?: (context: TransitionContext) => void
  complete?: (value?: unknown) => void
  cancel?: () => void
}

export interface TransitionContext {
  generation: number
  plan: TransitionPlan
  isCurrent: () => boolean
  requestFrame: (callback: (timestamp: number) => void) => number
  scheduleFallback: (callback: () => void, delayMs: number) => number
  listenTransitionEnd: (target: EventTarget, callback: (event: Event) => void) => () => void
  finish: (value?: unknown) => boolean
  cancel: () => boolean
}

interface ActiveTransition {
  generation: number
  plan: TransitionPlan
  callbacks: PresentationCallbacks
  finished: boolean
}

export interface PendingFrame {
  frame: DesktopLyricsFrame
  options: { animate?: boolean }
}

export class PresentationCoordinator {
  readonly planner: PresentationPlanner
  currentFrame: DesktopLyricsFrame = normalizeFrame()
  private activeTransition: ActiveTransition | null = null
  private pendingFrame: PendingFrame | null = null
  private generation = 0
  private fallbackTimer = 0
  private animationFrameIds = new Set<number>()
  private listenerCleanups = new Set<() => void>()

  constructor(planner = new PresentationPlanner()) {
    this.planner = planner
  }

  get isTransitioning() {
    return this.activeTransition !== null
  }

  get activeGeneration() {
    return this.activeTransition?.generation ?? this.generation
  }

  plan(
    targetFrame: Partial<DesktopLyricsFrame>,
    options: { animateTransition?: boolean; reducedMotion?: boolean } = {}
  ) {
    return this.planner.plan(this.currentFrame, targetFrame, options)
  }

  setCurrentFrame(frame: Partial<DesktopLyricsFrame>) {
    this.currentFrame = normalizeFrame(frame)
    return this.currentFrame
  }

  queueLatest(frame: Partial<DesktopLyricsFrame>, options: { animate?: boolean } = {}) {
    this.pendingFrame = { frame: normalizeFrame(frame), options }
    return this.pendingFrame
  }

  takeLatest() {
    const pending = this.pendingFrame
    this.pendingFrame = null
    return pending
  }

  beginTransition(
    plan: Pick<TransitionPlan, 'kind' | 'durationMs'>,
    callbacks: PresentationCallbacks = {}
  ) {
    this.cancelTransition()
    const generation = ++this.generation
    const transition: ActiveTransition = {
      generation,
      plan: {
        kind: plan.kind,
        durationMs: plan.durationMs,
        from: this.currentFrame,
        to: this.currentFrame
      },
      callbacks,
      finished: false
    }
    this.activeTransition = transition

    const context: TransitionContext = {
      generation,
      plan: transition.plan,
      isCurrent: () => this.activeTransition?.generation === generation,
      requestFrame: (callback) => this.requestFrame(generation, callback),
      scheduleFallback: (callback, delayMs) => this.scheduleFallback(generation, callback, delayMs),
      listenTransitionEnd: (target, callback) =>
        this.listenTransitionEnd(generation, target, callback),
      finish: (value) => this.finishTransition(generation, value),
      cancel: () => this.cancelTransition(generation)
    }

    callbacks.start?.(context)
    return context
  }

  finishTransition(generation: number, value?: unknown) {
    const transition = this.activeTransition
    if (!transition || transition.generation !== generation || transition.finished) {
      return false
    }

    transition.finished = true
    this.clearTransitionResources()
    this.activeTransition = null
    transition.callbacks.complete?.(value)
    return true
  }

  cancelTransition(expectedGeneration: number | null = null) {
    const transition = this.activeTransition
    if (
      expectedGeneration !== null &&
      (!transition || transition.generation !== expectedGeneration)
    ) {
      return false
    }

    transition?.callbacks.cancel?.()
    this.generation++
    this.clearTransitionResources()
    this.activeTransition = null
    return transition !== null
  }

  requestFrame(generation: number, callback: (timestamp: number) => void) {
    const frameId = window.requestAnimationFrame((timestamp) => {
      this.animationFrameIds.delete(frameId)
      if (this.activeTransition?.generation !== generation) return
      callback(timestamp)
    })
    this.animationFrameIds.add(frameId)
    return frameId
  }

  scheduleFallback(generation: number, callback: () => void, delayMs: number) {
    this.clearFallbackTimer()
    this.fallbackTimer = window.setTimeout(
      () => {
        this.fallbackTimer = 0
        if (this.activeTransition?.generation !== generation) return
        callback()
      },
      Math.max(0, Number(delayMs) || 0)
    )
    return this.fallbackTimer
  }

  listenTransitionEnd(generation: number, target: EventTarget, callback: (event: Event) => void) {
    const listener = (event: Event) => {
      if (this.activeTransition?.generation !== generation) return
      callback(event)
    }
    target.addEventListener('transitionend', listener)
    const cleanup = () => target.removeEventListener('transitionend', listener)
    this.listenerCleanups.add(cleanup)
    return cleanup
  }

  clearTransitionResources() {
    this.clearFallbackTimer()
    this.clearAnimationFrames()
    this.clearTransitionListeners()
  }

  clearAnimationFrames() {
    this.animationFrameIds.forEach((frameId) => window.cancelAnimationFrame(frameId))
    this.animationFrameIds.clear()
  }

  clearTransitionListeners() {
    this.listenerCleanups.forEach((cleanup) => cleanup())
    this.listenerCleanups.clear()
  }

  clearFallbackTimer() {
    if (!this.fallbackTimer) return
    window.clearTimeout(this.fallbackTimer)
    this.fallbackTimer = 0
  }
}
