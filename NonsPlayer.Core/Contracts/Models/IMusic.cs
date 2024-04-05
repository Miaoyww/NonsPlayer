using System.Text.Json.Serialization;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Models;

public class IMusic : INonsModel
{
    [JsonPropertyName("album")] public Album Album { get; set; }
    [JsonPropertyName("artists")] public Artist[] Artists { get; set; }
    [JsonPropertyName("is_empty")] public bool IsEmpty { get; set; }
    [JsonPropertyName("duration")] public TimeSpan Duration { get; set; }
    [JsonPropertyName("uri")] public string Uri { get; set; }
    [JsonPropertyName("lyric")] public Lyric Lyric { get; set; }
    [JsonIgnore] public byte[]? LocalCover { get; set; }
    [JsonIgnore] public string AlbumName => Album?.Name;
    [JsonIgnore] public string TotalTimeString => Duration.ToString(@"m\:ss");
    [JsonIgnore] public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
}