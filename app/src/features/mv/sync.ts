export const localMvDriftToleranceSeconds = 0.35
export const localMvOffsetStepMs = 100
export const maxLocalMvOffsetMs = 10 * 60 * 1000

export function clampLocalMvOffset(value: number) {
  if (!Number.isFinite(value)) return 0
  return Math.max(-maxLocalMvOffsetMs, Math.min(maxLocalMvOffsetMs, Math.round(value)))
}

export function localMvWaiting(positionMs: number, offsetMs: number) {
  return offsetMs > 0 && positionMs < offsetMs
}

export function localMvTargetSeconds(positionMs: number, durationSeconds: number, offsetMs = 0) {
  const target = Math.max(0, (positionMs - offsetMs) / 1000)
  return Number.isFinite(durationSeconds) && durationSeconds > 0
    ? Math.min(target, durationSeconds)
    : target
}

export function localMvNeedsSync(
  currentSeconds: number,
  targetSeconds: number,
  tolerance = localMvDriftToleranceSeconds
) {
  return (
    Number.isFinite(currentSeconds) &&
    Number.isFinite(targetSeconds) &&
    Math.abs(currentSeconds - targetSeconds) > tolerance
  )
}
