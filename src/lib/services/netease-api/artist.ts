/**
 * Artist module — artist detail with hot songs.
 *
 * Mirror of api-enhanced module: artists
 */

import type { Artist } from "$lib/types/artist";
import { mapArtist, mapSong } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function getArtist(id: string): Promise<Artist> {
  const rawId = id.replace(/^netease_artist_/, "");
  const data = await neteaseRequest("api", `/api/v1/artist/${rawId}`, {});
  const artist = data.artist ?? data;
  const hotSongs: any[] = data.hotSongs ?? artist.hotSongs ?? [];
  return {
    ...mapArtist(artist),
    songs: hotSongs.map(mapSong),
    musicCount: artist.musicSize ?? hotSongs.length,
  };
}
