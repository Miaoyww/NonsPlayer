using System.Collections.Generic;

namespace NcmPlayer.Framework.Model
{
    public class Artist
    {
        /// <summary>
        /// 歌手Id
        /// </summary>
        public long Id { get; set; }

        /// <summary>
        /// 歌手名
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// 歌手图片
        /// </summary>
        public string PicUrl { get; set; }

        /// <summary>
        /// mv数量
        /// </summary>
        public int MvCount { get; set; }

        /// <summary>
        /// 音乐数量
        /// </summary>
        public int MusicCount { get; set; }

        /// <summary>
        /// 专辑数量
        /// </summary>
        public int AlbumCount { get; set; }

        /// <summary>
        /// 如果歌手有账户，对应账户id
        /// </summary>
        public long AccountId { get; set; }

        /// <summary>
        /// 歌手的热门歌曲
        /// </summary>
        public List<Music> HotMusics { get; set; }
    }
}
