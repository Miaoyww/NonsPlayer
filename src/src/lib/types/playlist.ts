import type { SongModel } from "./base";
import type { Song } from "./song";

export interface Playlist extends SongModel {
  title: string;
  createTime: string;
  creator: string;
  description: string;
  musicTrackIds: string[];
  tags: string[];
  musics: Song[];
  isInitialized: boolean;
  playCount: number;
  /** musicTrackIds.length（computed） */
  musicsCount: number;
  /** 来源适配器标识 */
  adapterSlug: string;
}
