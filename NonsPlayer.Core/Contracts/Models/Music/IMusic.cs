using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Nons;
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
    [JsonPropertyName("available")] public bool Available { get; set; }
    [JsonIgnore] string AlbumName => Album?.Name;
    [JsonIgnore] string TotalTimeString => Duration.ToString(@"m\:ss");
    [JsonIgnore] string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    [JsonIgnore] IAdapter Adapter { get; set; }

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

    /// <summary>
    /// 获取封面URL
    /// </summary>
    /// <param name="param">可选参数</param>
    /// <returns></returns>
    string GetCoverUrl(string param = "");

    /// <summary>
    /// 收藏歌曲
    /// </summary>
    /// <returns>成功状态</returns>
    Task<bool> Like(bool like);

    /// <summary>
    /// 获取收藏状态
    /// </summary>
    /// <returns>收藏状态</returns>
    Task<bool> GetLikeState(); 

    /// <summary>
    /// 获取可用状态, 调用时会对Available赋值
    /// </summary>
    /// <returns>可用状态</returns>
    Task<bool> GetAvailable();
}