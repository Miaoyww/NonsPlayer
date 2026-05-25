/** 基础模型 — 所有模型共有字段 */
export interface NonsModel {
  id: string;
  md5: string;
  name: string;
}

/** 音乐模型中间层 — 专辑/艺术家/歌曲/歌单共有字段 */
export interface SongModel extends NonsModel {
  shareUrl: string;
  avatarUrl: string;
  /** avatarUrl + "?param=50y50" */
  smallAvatarUrl: string;
  /** avatarUrl + "?param=200y200" */
  middleAvatarUrl: string;
}
