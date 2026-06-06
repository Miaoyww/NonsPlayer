/**
 * Search module — standard search.
 *
 * Mirror of api-enhanced module: search
 *
 * POST /api/search/get  (weapi)
 *
 * type: 1=单曲 10=专辑 100=歌手 1000=歌单 1002=用户 1004=MV 1006=歌词 1009=电台 1014=视频
 */

import type { SearchResult } from "$lib/types/adapter";
import { mapSearchResult } from "../netease-mapper";
import { neteaseRequest } from "./request";

export async function search(
  keyword: string,
  type: number = 1,
  limit: number = 20,
  offset: number = 0,
): Promise<SearchResult> {
  const data = await neteaseRequest("api", "/api/search/get", {
    s: keyword,
    type: String(type),
    limit: String(limit),
    offset: String(offset),
  });
  return mapSearchResult(data);
}
