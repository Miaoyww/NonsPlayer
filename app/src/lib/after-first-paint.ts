// Let the initial shell paint before optional work. Later interactions run immediately.
let painted = false
export function afterFirstPaint(work: () => void): () => void {
  if (painted || typeof window === 'undefined' || !window.requestAnimationFrame) {
    work()
    return () => {}
  }
  let cancelled = false
  let frame = 0
  const run = () => {
    if (cancelled) return
    cancelled = true
    painted = true
    window.cancelAnimationFrame(frame)
    window.clearTimeout(timeout)
    work()
  }
  // A hidden window may not get frames; optional startup work must still progress.
  const timeout = window.setTimeout(run, 200)
  frame = window.requestAnimationFrame(() => {
    frame = window.requestAnimationFrame(run)
  })
  return () => {
    cancelled = true
    window.cancelAnimationFrame(frame)
    window.clearTimeout(timeout)
  }
}
