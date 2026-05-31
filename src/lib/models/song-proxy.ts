import type { Song } from "$lib/types/song";
import { getSongUrl, getLyric, toggleLike } from "$lib/services/adapter-service";

/**
 * Wraps a Song data object with convenience methods
 * that invoke the correct adapter automatically.
 */
export class SongProxy {
  constructor(public data: Song) {}

  get id() { return this.data.id; }
  get name() { return this.data.name; }
  get duration() { return this.data.duration; }
  get durationText() { return this.data.durationText; }
  get albumName() { return this.data.albumName; }
  get artistsName() { return this.data.artistsName; }
  get adapterSlug() { return this.data.adapterSlug; }
  get isLiked() { return this.data.isLiked; }
  get available() { return this.data.available; }
  get album() { return this.data.album; }
  get artists() { return this.data.artists; }
  get avatarUrl() { return this.data.avatarUrl; }
  get smallAvatarUrl() { return this.data.smallAvatarUrl; }

  async getUrl(): Promise<string> {
    return getSongUrl(this.adapterSlug, this.id);
  }

  async getLyric(): Promise<string> {
    return getLyric(this.adapterSlug, this.id);
  }

  async like(like: boolean): Promise<boolean> {
    const result = await toggleLike(this.adapterSlug, this.id, like);
    this.data.isLiked = like;
    return result;
  }
}
