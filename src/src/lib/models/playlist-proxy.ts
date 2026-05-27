import type { Playlist } from "$lib/types/playlist";
import { getPlaylist } from "$lib/services/adapter-service";
import type { Song } from "$lib/types/song";
import { SongProxy } from "./song-proxy";

/**
 * Wraps a Playlist data object with convenience methods.
 */
export class PlaylistProxy {
  constructor(public data: Playlist) {}

  get id() { return this.data.id; }
  get name() { return this.data.name; }
  get title() { return this.data.title; }
  get description() { return this.data.description; }
  get creator() { return this.data.creator; }
  get adapterSlug() { return this.data.adapterSlug; }
  get avatarUrl() { return this.data.avatarUrl; }
  get songs(): Song[] { return this.data.musics; }
  get musicsCount() { return this.data.musicsCount; }

  /** Fetch full playlist details from the adapter. */
  async refresh(): Promise<Playlist> {
    this.data = await getPlaylist(this.adapterSlug, this.id);
    return this.data;
  }

  /** Get SongProxy wrappers for all songs. */
  getSongProxies(): SongProxy[] {
    return this.songs.map((s) => new SongProxy(s));
  }
}
