export type ListeningStatistics = {
  seconds: number
  days: number
  plays: number
}

export function formatListeningDuration(seconds: number) {
  const total = Math.max(0, Math.floor(seconds))
  if (total < 60) return `${total} 秒`
  const minutes = Math.floor(total / 60)
  if (minutes < 60) return `${minutes} 分钟`
  const hours = Math.floor(minutes / 60)
  const rest = minutes % 60
  return rest ? `${hours} 小时 ${rest} 分` : `${hours} 小时`
}

export function formatListeningCount(value: number) {
  return Math.max(0, Math.floor(value)).toLocaleString('en-US')
}
