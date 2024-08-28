using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models;
using System.Text.Json.Serialization;

namespace NonsPlayer.Models;

public class AppSettings
{
    #region Player Settings

    /// <summary>
    ///     是否显示翻译歌词
    /// </summary>
    [JsonPropertyName("is_show_tran_lyric")]
    public bool IsShowTranLyric { get; set; }

    /// <summary>
    ///     是否启用系统媒体控制
    /// </summary>
    [JsonPropertyName("smtc_enable")]
    public bool SMTCEnable { get; set; }

    /// <summary>
    ///     加载下一页歌单的偏移量
    /// </summary>
    [JsonPropertyName("playlist_load_offset")]
    public double PlaylistLoadOffset { get; set; }

    /// <summary>
    ///     歌单详情页一次显示的歌曲数量
    /// </summary>
    [JsonPropertyName("playlist_track_count")]
    public int PlaylistTrackCount { get; set; }

    /// <summary>
    ///     主页推荐歌单数量
    /// </summary>
    [JsonPropertyName("home_recommended_count")]
    public int RecommendedPlaylistCount { get; set; }

    /// <summary>
    ///     按一下音量加减的增量
    /// </summary>
    [JsonPropertyName("volume_step")]
    public double VolumeStep { get; set; }

    #endregion

    #region Remote Settings

    /// <summary>
    ///     远程控制端口
    /// </summary>
    [JsonPropertyName("api_port")]
    public int ApiPort { get; set; }

    #endregion

    public AppSettings()
    {
        IsShowTranLyric = true;
        SMTCEnable = true;
        ApiPort = 8080;
        PlaylistLoadOffset = 500;
        PlaylistTrackCount = 30;
        VolumeStep = 10;
        RecommendedPlaylistCount = 20;
    }
}