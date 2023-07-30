using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models
{
    public class Album
    {
        [JsonPropertyName("id")] public int Id;
        public string CacheCoverId => Id.ToString() + "_album_cover";
        public string CacheSmallCoverId => Id.ToString() + "_album_small_cover";

        [JsonPropertyName("name")] public string Name;

        [JsonPropertyName("pic_url")] public string CoverUrl;

        public string SmallCoverUrl;

        [JsonPropertyName("create_date")] public DateTime CreateDate;
        [JsonPropertyName("description")] public string Description;

        /// <summary>
        /// 专辑歌曲
        /// </summary>
        public List<Music> Musics
        {
            get;
            set;
        }

        /// <summary>
        /// 专辑作者
        /// </summary>
        public List<Artist> Artists
        {
            get;
            set;
        }

        [JsonPropertyName("artists_name")]
        public string ArtistsName
        {
            get
            {
                if (Artists == null) return string.Empty;
                return string.Join("/", Artists.Select(x => x.Name));
            }
        }

        /// <summary>
        /// 收藏数目
        /// </summary>
        public int CollectionCount
        {
            set;
            get;
        }

        /// <summary>
        /// 歌曲数目
        /// </summary>
        public int TrackCount
        {
            get;
            set;
        }
    }
}