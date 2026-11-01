import { DomLyricPlayer } from '@applemusic-like-lyrics/core'
import { ManualScrollLock } from './manual-lyric-scroll'

export class ManualLyricPlayer extends DomLyricPlayer {
  private readonly manualScroll = new ManualScrollLock(() => this.resetScroll())
  private readonly onManualScroll = () => this.manualScroll.suspend()

  constructor() {
    super()
    this.element.addEventListener('wheel', this.onManualScroll, {
      capture: true,
      passive: true
    })
    this.element.addEventListener('touchmove', this.onManualScroll, {
      capture: true,
      passive: true
    })
  }

  releaseManualScroll() {
    this.manualScroll.release()
    super.resetScroll()
  }

  override resetScroll() {
    if (this.manualScroll.isLocked) return
    super.resetScroll()
  }

  override dispose() {
    this.element.removeEventListener('wheel', this.onManualScroll, true)
    this.element.removeEventListener('touchmove', this.onManualScroll, true)
    this.manualScroll.dispose()
    super.dispose()
  }
}
