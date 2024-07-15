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

public interface IMusic : IMusicModel
{
    [JsonPropertyName("name")] public string Name { get; set; }
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

    /// <summary>
    /// 是否已收藏
    /// </summary>
    bool IsLiked { get; set; }

    /// <summary>
    /// 翻译、说明
    /// </summary>
    string? Trans { get; set; }

    /// <summary>
    /// 获取歌曲播放地址
    /// </summary>
    /// <param name="quality">音乐品质</param>
    /// <returns>歌曲URL</returns>
    Task<string> GetUrl(MusicQualityLevel quality = MusicQualityLevel.Standard);

    /// <summary>
    /// 获取歌曲歌词
    /// </summary>
    /// <returns>歌词</returns>
    Task<Lyric> GetLyric();
}