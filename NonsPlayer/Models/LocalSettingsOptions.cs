using Microsoft.UI.Xaml.Media;

namespace NonsPlayer.Models;

public class LocalSettingsOptions
{
    public string? ApplicationDataFolder { get; set; }

    public string? LocalSettingsFile { get; set; }

    /// <summary>
    /// 播放歌单内音乐直接将歌曲添加至播放列表中
    /// </summary>
    public bool IsPlay2List = true;

    /// <summary>
    /// 远程控制端口
    /// </summary>
    public int ApiPort = 1217;

    /// <summary>
    /// 是否启用系统媒体控制
    /// </summary>
    public bool MediaControl = true;

    /// <summary>
    /// 按一下音量加减的增量
    /// </summary>
    public double VolumeAddition = 5;

    /// <summary>
    /// 主页推荐歌单数量
    /// </summary>
    public int RecommendedPlaylistCount = 20;

    /// <summary>
    /// 歌单详情页一次显示的歌曲数量
    /// </summary>
    public int PlaylistTrackShowCount = 30;

    /// <summary>
    /// 加载下一页歌单的偏移量
    /// </summary>
    public double PlaylistLoadOffset = 500;
    
    /// <summary>
    /// 是否显示翻译歌词
    /// </summary>
    public bool IsShowTranLyric = true;
    
    /// <summary>
    /// 歌词字体
    /// </summary>
    public FontFamily LyricFontFamily;
}