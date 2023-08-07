using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models;

public class Artist
{
    [JsonPropertyName("account_id")] public long AccountId;

    public int AlbumCount;

    public List<Music> HotMusics;
    [JsonPropertyName("id")] public long Id;

    public int MusicCount;

    public int MvCount;

    [JsonPropertyName("name")] public string Name;

    [JsonPropertyName("pic_url")] public string PicUrl;
    public string CacheId => Id + "_artist";
}