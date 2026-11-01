export const MANUAL_LYRIC_SCROLL_RESUME_DELAY_MS = 3500

type Timer = ReturnType<typeof setTimeout>
type Schedule = (callback: () => void, delay: number) => Timer
type Cancel = (timer: Timer) => void

export class ManualScrollLock {
  private timer: Timer | undefined
  private locked = false

  constructor(
    private readonly onResume: () => void,
    private readonly delay = MANUAL_LYRIC_SCROLL_RESUME_DELAY_MS,
    private readonly schedule: Schedule = (callback, timeout) => setTimeout(callback, timeout),
    private readonly cancel: Cancel = (timer) => clearTimeout(timer)
  ) {}

  get isLocked() {
    return this.locked
  }

  suspend() {
    this.locked = true
    if (this.timer !== undefined) this.cancel(this.timer)
    this.timer = this.schedule(() => {
      this.timer = undefined
      this.locked = false
      this.onResume()
    }, this.delay)
  }

  release() {
    if (this.timer !== undefined) {
      this.cancel(this.timer)
      this.timer = undefined
    }
    this.locked = false
  }

  dispose() {
    this.release()
  }
}
