import type { SongModel } from "./base";
import type { Album } from "./album";
import type { Artist } from "./artist";

export interface Song extends SongModel {
  album: Album;
  artists: Artist[];
  isEmpty: boolean;
  /** 时长（秒） */
  duration: number;
  url: string;
  lyric: unknown;
  available: boolean;
  isLiked: boolean;
  trans: string | null;
  /** 专辑名（computed） */
  albumName: string;
  /** 艺术家名，以 "/" 连接（computed） */
  artistsName: string;
  /** 格式化时长 "m:ss"（computed） */
  durationText: string;
}
