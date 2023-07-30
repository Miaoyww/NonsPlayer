using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models
{
    public class Artist
    {
        [JsonPropertyName("id")] public long Id;
        public string CacheId => Id.ToString() + "_artist";

        [JsonPropertyName("name")] public string Name;

        [JsonPropertyName("pic_url")] public string PicUrl;

        public int MvCount;

        public int MusicCount;

        public int AlbumCount;

        [JsonPropertyName("account_id")] public long AccountId;

        public List<Music> HotMusics;
    }
}