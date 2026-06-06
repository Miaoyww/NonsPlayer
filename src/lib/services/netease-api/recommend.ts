/**
 * Recommend module — personalized playlists + daily song recommendations.
 *
 * Mirror of api-enhanced modules:
 *   personalized, recommend_songs
 */

import type { Song } from "$lib/types/song";
import type { Playlist } from "$lib/types/playlist";
import { mapSong, mapPlaylistsFromList } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function getRecommendedPlaylists(count: number = 30): Promise<Playlist[]> {
  const data = await neteaseRequest("api", "/api/personalized/playlist", {
    limit: String(count),
    total: "true",
    n: "1000",
  });
  return mapPlaylistsFromList(data.result ?? []);
}

export async function getDailyRecommended(): Promise<Song[]> {
  const data = await neteaseRequest("api", "/api/v3/discovery/recommend/songs", {});
  const dailySongs: any[] = data.data?.dailySongs ?? [];
  return dailySongs.map(mapSong);
}
