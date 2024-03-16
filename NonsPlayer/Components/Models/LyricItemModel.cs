using System.ComponentModel;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class SongLyric
{
    public static SongLyric PureSong = new()
        { PureLine = new LrcLyricsLine("纯音乐 请欣赏", string.Empty, TimeSpan.Zero) };

    public static SongLyric NoLyricItem = new()
        { PureLine = new LrcLyricsLine("无歌词 请欣赏", string.Empty, TimeSpan.Zero) };

    public static SongLyric LoadingLyricItem = new()
        { PureLine = new LrcLyricsLine("加载歌词中...", string.Empty, TimeSpan.Zero) };

    public ILyricLine PureLine;
    public string Translation;
    public bool HaveTranslation => !string.IsNullOrEmpty(Translation);
    public Type LyricType => PureLine.GetType();
    public Visibility TransVisibility = Visibility.Visible;
}

public class LyricItemModel
{
    private bool isShow;

    public LyricItemModel(SongLyric songLyric, int index)
    {
        SongLyric = songLyric;
        Index = index;
    }

    public SongLyric SongLyric { get; }

    public int Index { get; }

}