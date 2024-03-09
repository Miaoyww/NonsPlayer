using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class LyricModel
{
    public static LyricModel PureSong = new()
        { LyricLine = new LrcLyricsLine("纯音乐 请欣赏", string.Empty, TimeSpan.Zero) };

    public static LyricModel NoLyric = new()
        { LyricLine = new LrcLyricsLine("无歌词 请欣赏", string.Empty, TimeSpan.Zero) };

    public static LyricModel LoadingLyric = new()
        { LyricLine = new LrcLyricsLine("加载歌词中...", string.Empty, TimeSpan.Zero) };

    public ILyricLine LyricLine;
    public string Translation;
    public Thickness Margin;
    public bool HaveTranslation => !string.IsNullOrEmpty(Translation);
    public Type LyricType => LyricLine.GetType();
    public Visibility TransVisibility;
}