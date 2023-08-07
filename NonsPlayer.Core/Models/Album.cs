using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models;

public class Album
{
    [JsonPropertyName("pic_url")] public string CoverUrl;

    [JsonPropertyName("create_date")] public DateTime CreateDate;
    [JsonPropertyName("description")] public string Description;
    [JsonPropertyName("id")] public int Id;

    [JsonPropertyName("name")] public string Name;

    public string SmallCoverUrl;
    public string CacheCoverId => Id + "_album_cover";
    public string CacheSmallCoverId => Id + "_album_small_cover";

    /// <summary>
    ///     专辑歌曲
    /// </summary>
    public List<Music> Musics { get; set; }

    /// <summary>
    ///     专辑作者
    /// </summary>
    public List<Artist> Artists { get; set; }

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
    ///     收藏数目
    /// </summary>
    public int CollectionCount { set; get; }

    /// <summary>
    ///     歌曲数目
    /// </summary>
    public int TrackCount { get; set; }
}