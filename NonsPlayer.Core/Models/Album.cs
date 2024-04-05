using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class Album : INonsModel
{
    [JsonPropertyName("create_date")] public DateTime CreateDate;
    [JsonPropertyName("description")] public string Description;
    [JsonPropertyName("musics")] public List<Music> Musics { get; set; }
    [JsonPropertyName("artists")] public List<Artist> Artists { get; set; }
    [JsonIgnore] public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    [JsonPropertyName("collection_count")] public int CollectionCount { set; get; }
    [JsonPropertyName("track_count")] public int TrackCount { get; set; }
}