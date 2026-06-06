/**
 * Discover module — toplist, playlist categories, playlist square.
 *
 * Mirror of api-enhanced modules:
 *   toplist, playlist_catlist, top_playlist, top_playlist_highquality
 */

import type { Playlist } from "$lib/types/playlist";
import type { TopPlaylistGroup } from "$lib/types/top-playlist";
import type { PlaylistCategory } from "$lib/types/playlist-category";
import { mapTopPlaylistGroups, mapPlaylistCategories, mapPlaylistsFromList } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function getTopPlaylists(): Promise<TopPlaylistGroup[]> {
  const data = await neteaseRequest("api", "/api/toplist", {});
  return mapTopPlaylistGroups(data.list ?? []);
}

export async function getPlaylistCats(): Promise<PlaylistCategory[]> {
  const data = await neteaseRequest("api", "/api/playlist/catalogue", {});
  return mapPlaylistCategories(data);
}

export async function getPlaylistSquare(
  cat: string,
  order: string,
  limit: number,
  offset: number,
  highQuality: boolean,
): Promise<{ playlists: Playlist[]; total: number }> {
  if (highQuality) {
    const data = await neteaseRequest("api", "/api/playlist/highquality/list", {
      cat,
      limit: String(limit),
      lasttime: String(offset),
      total: "true",
    });
    const playlists = mapPlaylistsFromList(data.playlists ?? []);
    return { playlists, total: data.total ?? 0 };
  } else {
    const data = await neteaseRequest("api", "/api/playlist/list", {
      cat,
      order,
      limit: String(limit),
      offset: String(offset),
      total: "true",
    });
    const playlists = mapPlaylistsFromList(data.playlists ?? []);
    return { playlists, total: data.total ?? 0 };
  }
}
