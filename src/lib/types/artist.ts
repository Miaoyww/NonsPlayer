import type { SongModel } from "./base";
import type { Song } from "./song";

export interface Artist extends SongModel {
  description: string;
  songs: Song[];
  musicCount: number;
  trans: string;
  /** 来源适配器标识 */
  adapterSlug: string;
}
