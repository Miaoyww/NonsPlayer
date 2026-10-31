const completedKey = 'nons-welcome-completed-v1'
export const openWelcomeEvent = 'nons-open-welcome'

export function hasCompletedWelcome() {
  try {
    return globalThis.localStorage?.getItem(completedKey) === 'true'
  } catch {
    return false
  }
}

export function markWelcomeCompleted() {
  try {
    globalThis.localStorage?.setItem(completedKey, 'true')
  } catch {
    // The page can still be dismissed for this session when storage is unavailable.
  }
}

export function reopenWelcome() {
  try {
    globalThis.localStorage?.removeItem(completedKey)
  } catch {
    // Opening the page is still useful even when the preference cannot be persisted.
  }
  window.dispatchEvent(new Event(openWelcomeEvent))
}
