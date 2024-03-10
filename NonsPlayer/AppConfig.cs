using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Activation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;
using NonsPlayer.Views.Pages;
using WinRT;
using UnhandledExceptionEventArgs = Microsoft.UI.Xaml.UnhandledExceptionEventArgs;

namespace NonsPlayer;

internal static class AppConfig
{
    public static string LogFile { get; private set; }

    static AppConfig()
    {
        Initialize();
    }

    public static string? IgnoreVersion { get; set; }
    public static string? AppVersion { get; set; }

    private static void Initialize()
    {
#if DEBUG
        AppVersion = "0.4.1";
#else
        AppVersion = typeof(AppConfig).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion;
#endif
        AppVersion = "0.4.1";
    }

    #region Player Settings

    /// <summary>
    ///     播放歌单内音乐直接将歌曲添加至播放列表中
    /// </summary>
    public static bool IsPlay2List = true;

    /// <summary>
    ///     是否显示翻译歌词
    /// </summary>
    public static bool IsShowTranLyric = true;

    /// <summary>
    ///     歌词字体
    /// </summary>
    public static FontFamily LyricFontFamily;

    /// <summary>
    ///     是否启用系统媒体控制
    /// </summary>
    public static bool MediaControl = true;

    /// <summary>
    ///     加载下一页歌单的偏移量
    /// </summary>
    public static double PlaylistLoadOffset = 500;

    /// <summary>
    ///     歌单详情页一次显示的歌曲数量
    /// </summary>
    public static int PlaylistTrackShowCount = 30;

    /// <summary>
    ///     主页推荐歌单数量
    /// </summary>
    public static int RecommendedPlaylistCount = 20;

    /// <summary>
    ///     按一下音量加减的增量
    /// </summary>
    public static double VolumeAddition = 10;

    #endregion

    #region Remote Settings

    /// <summary>
    ///     远程控制端口
    /// </summary>
    public static int ApiPort = 8080;

    #endregion
}