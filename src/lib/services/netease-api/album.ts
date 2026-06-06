/**
 * Album module — album detail with songs.
 *
 * Mirror of api-enhanced module: album
 */

import type { Album } from "$lib/types/album";
import { mapAlbum, mapSong } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function getAlbum(id: string): Promise<Album> {
  const rawId = id.replace(/^netease_album_/, "");
  const data = await neteaseRequest("api", `/api/v1/album/${rawId}`, {});
  const album = data.album ?? data;
  const songs: any[] = data.songs ?? album.songs ?? [];
  return {
    ...mapAlbum(album),
    songs: songs.map(mapSong),
  };
}
