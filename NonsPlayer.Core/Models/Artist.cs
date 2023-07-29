using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models
{
    public class Artist
    {
       [JsonPropertyName("id")]
        public long Id
        {
            get; set;
        }

        [JsonPropertyName("name")]
        public string Name
        {
            get; set;
        }

        [JsonPropertyName("pic_url")]
        public string PicUrl
        {
            get; set;
        }
        
        public int MvCount
        {
            get; set;
        }
        
        public int MusicCount
        {
            get; set;
        }
        
        public int AlbumCount
        {
            get; set;
        }
        
        [JsonPropertyName("account_id")]
        public long AccountId
        {
            get; set;
        }
        
        public List<Music> HotMusics
        {
            get; set;
        }
    }
}