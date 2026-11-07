import { MusicNavigationItems } from './music-navigation-items'

export function MobileNavigation() {
  return (
    <nav className="mobile-navigation" aria-label="音乐导航">
      <MusicNavigationItems mobile />
    </nav>
  )
}
