import { usePrimaryMusicPage } from './use-primary-music-page'

export function AndroidTitlebar({ playerMode = false }: { playerMode?: boolean }) {
  const primaryPage = usePrimaryMusicPage()
  if (playerMode || !primaryPage) return null
  return <header className="android-titlebar" aria-hidden="true" />
}
