using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models
{
    public class Album
    {
        [JsonPropertyName("id")]
        public int Id
        {
            get;
            set;
        }

        [JsonPropertyName("name")]
        public string Name
        {
            get;
            set;
        }

        [JsonPropertyName("pic_url")]
        public string CoverUrl
        {
            get;
            set;
        }

        public string SmallCoverUrl
        {
            get;
            set;
        }

        [JsonPropertyName("create_date")]
        public DateTime CreateDate
        {
            get;
            set;
        }

        [JsonPropertyName("description")]
        public string Description
        {
            get;
            set;
        }

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