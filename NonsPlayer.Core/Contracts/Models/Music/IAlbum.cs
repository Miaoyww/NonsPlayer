using NonsPlayer.Core.Contracts.Adapters;
using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Contracts.Models.Music;

public interface IAlbum : IMusicModel
{
    [JsonPropertyName("create_date")] public DateTime CreateDate { get; set; }
    [JsonPropertyName("description")] public string Description { get; set; }
    [JsonPropertyName("musics")] public IMusic[] Musics { get; set; }
    [JsonPropertyName("artists")] public IArtist[] Artists { get; set; }
    [JsonIgnore] public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    [JsonPropertyName("collection_count")] public int CollectionCount { set; get; }
    [JsonPropertyName("track_count")] public int TrackCount { get; set; }
    [JsonIgnore] IAdapter Adapter { get; set; }
}