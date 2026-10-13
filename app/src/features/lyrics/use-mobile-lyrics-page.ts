import { useEffect, useState } from 'react'
import { isAndroid } from '@/lib/platform'

export function useMobileLyricsPage(present: boolean) {
  const mobile = isAndroid()
  const [lyricsPage, setLyricsPage] = useState(false)
  useEffect(() => {
    if (!present) setLyricsPage(false)
  }, [present])
  useEffect(() => {
    if (!mobile || !lyricsPage || !present) return
    const back = (event: Event) => {
      event.preventDefault()
      setLyricsPage(false)
    }
    window.addEventListener('nons-android-back', back)
    return () => window.removeEventListener('nons-android-back', back)
  }, [mobile, lyricsPage, present])
  return {
    mobile,
    lyricsPage,
    showLyricsPage: () => setLyricsPage(true),
    showOverview: () => setLyricsPage(false)
  }
}
