using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Contracts.Models.Music;

public interface IMusic: IMusicModel
{
    [JsonPropertyName("album")] public IAlbum Album { get; set; }
    [JsonPropertyName("artists")] public IArtist[] Artists { get; set; }
    [JsonPropertyName("is_empty")] public bool IsEmpty { get; set; }
    [JsonPropertyName("duration")] public TimeSpan Duration { get; set; }
    [JsonPropertyName("url")] public string Url { get; set; }
    [JsonPropertyName("lyric")] public Lyric Lyric { get; set; }
    [JsonIgnore] byte[]? LocalCover { get; set; }
    [JsonIgnore] string AlbumName => Album?.Name;
    [JsonIgnore] string TotalTimeString => Duration.ToString(@"m\:ss");
    [JsonIgnore] string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    bool IsLiked { get; set; }
    MusicQualityLevel[] QualityLevels { get; set; }
    string? Trans { get; set; }
    Task GetUrl();
    Task GetLyric();
}