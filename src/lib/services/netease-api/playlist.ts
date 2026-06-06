/**
 * Playlist module — playlist detail with tracks.
 *
 * Mirror of api-enhanced module: playlist_detail
 */

import type { Playlist } from "$lib/types/playlist";
import { mapPlaylist } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function getPlaylist(id: string): Promise<Playlist> {
  const rawId = id.replace(/^netease_playlist_/, "");
  const data = await neteaseRequest("api", "/api/v6/playlist/detail", {
    id: rawId,
    n: "100000",
    s: "8",
  });
  const pl = data.playlist ?? data;
  return mapPlaylist(pl);
}
