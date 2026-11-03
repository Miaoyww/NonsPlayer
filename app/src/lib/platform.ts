/** Android WebViews identify the OS even when the frontend is bundled offline. */
export function isAndroid() {
  return typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent)
}

export function isWindows() {
  return typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent)
}
