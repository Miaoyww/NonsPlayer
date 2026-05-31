import { convertFileSrc } from "@tauri-apps/api/core";
import { getSongUrl } from "$lib/services/adapter-service";
import type { Song } from "$lib/types/song";

/**
 * Information about a resolved audio source.
 */
export interface AudioSource {
  id: string;
  url?: string;
  isLocal?: boolean;
  source?: string;
}

/**
 * Convert a raw file path (possibly with `file://` prefix) into a URL
 * the webview can actually load. Tauri's `convertFileSrc` uses the
 * custom asset protocol to serve local files without `file://` restrictions.
 */
function toPlayableUrl(raw: string): string {
  // Strip file:// prefix if present
  const path = raw.startsWith("file://") ? raw.slice(7) : raw;
  return convertFileSrc(path);
}

/**
 * SongManager — resolves Song objects into playable audio URLs.
 *
 * Calls the Rust backend's adapter layer to get URLs from
 * NetEase Cloud Music (or other adapters), and handles local files
 * via Tauri's asset protocol.
 */
class SongManager {
  /** Prefetched audio source for the next song. */
  private nextPrefetch: AudioSource | null = null;

  /**
   * Resolve a Song to a playable URL.
   * Prioritizes: local files → prefetch cache → adapter API.
   */
  async getAudioSource(song: Song): Promise<AudioSource> {
    // Local files — use Tauri asset protocol
    if (song.url && song.url.startsWith("file://")) {
      const assetUrl = toPlayableUrl(song.url);
      return { id: song.id, url: assetUrl, isLocal: true, source: "local" };
    }

    // Prefetch cache hit
    if (this.nextPrefetch && this.nextPrefetch.id === song.id) {
      const cached = this.nextPrefetch;
      this.nextPrefetch = null;
      return cached;
    }

    // Remote — call adapter (frontend HTTP for netease, Rust otherwise)
    try {
      const url = await getSongUrl(song.adapterSlug, song.id);

      if (!url) {
        console.warn(`[SongManager] No URL returned for "${song.name}" (${song.id})`);
        return { id: song.id, url: undefined };
      }

      if (url.startsWith("file://")) {
        const assetUrl = toPlayableUrl(url);
        return { id: song.id, url: assetUrl, isLocal: true, source: "local" };
      }

      return { id: song.id, url, isLocal: false, source: song.adapterSlug };
    } catch (e) {
      console.error(`[SongManager] Failed to get URL for "${song.name}":`, e);
      return { id: song.id, url: undefined };
    }
  }

  /**
   * Prefetch the next song's URL so it's ready when the current song ends.
   */
  async prefetchNextSong(song: Song | null): Promise<void> {
    if (!song) {
      this.nextPrefetch = null;
      return;
    }

    // Local files are instant — store the converted URL
    if (song.url && song.url.startsWith("file://")) {
      this.nextPrefetch = {
        id: song.id,
        url: toPlayableUrl(song.url),
        isLocal: true,
        source: "local",
      };
      return;
    }

    try {
      const url = await getSongUrl(song.adapterSlug, song.id);
      if (url) {
        this.nextPrefetch = {
          id: song.id,
          url: url.startsWith("file://") ? toPlayableUrl(url) : url,
          isLocal: url.startsWith("file://"),
        };
      }
    } catch (e) {
      console.warn(`[SongManager] Prefetch failed for "${song.name}":`, e);
    }
  }

  /** Clear the prefetch cache (e.g., on error or queue clear). */
  clearPrefetch(): void {
    this.nextPrefetch = null;
  }
}

let instance: SongManager | null = null;

export const useSongManager = (): SongManager => {
  if (!instance) instance = new SongManager();
  return instance;
};
