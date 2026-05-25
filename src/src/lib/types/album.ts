import type { SongModel } from "./base";
import type { Song } from "./song";
import type { Artist } from "./artist";

export interface Album extends SongModel {
  createDate: string;
  description: string;
  songs: Song[];
  artists: Artist[];
  /** 艺术家名，以 "/" 连接（computed） */
  artistsName: string;
  collectionCount: number;
  trackCount: number;
}
