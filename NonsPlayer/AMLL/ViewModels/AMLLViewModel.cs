using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.AMLL.Models;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.AMLL.ViewModels;

public partial class AMLLViewModel : ObservableRecipient
{
    public ObservableCollection<LyricCombiner> LyricItems = new();
    [ObservableProperty] private Music currentMusic;
    public int LyricPosition;
    
    #region 命令

    [RelayCommand]
    public void SwitchPlayMode()
    {
        
    }

    [RelayCommand]
    public void SwitchShuffle()
    {
        
    }

    #endregion

    public AMLLViewModel()
    {
        // LyricPositionGetter += OnLyricPositionGetter;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        
    }
    private async void OnMusicChanged(Music value)
    {
        LyricItems.Clear();
        try
        {
            for (int i = 0; i < value.Lyric.Count; i++)
            {
                LyricItems.Add(new LyricCombiner
                {
                    LyricItemModel = new LyricItemModel(value.Lyric.LyricLines[i]),
                    Index = i
                });
            }
        }
        catch (Exception e)
        {
            Console.WriteLine(e);
        }
     
        
    }
    // private void OnMusicChanged(Music music)
    // {
    //     CurrentMusic = music;
    //     LyricItems.Clear();
    //
    //     for (int i = 0; i < music.Lyrics.Lyrics.Lines.Count; i++)
    //     {
    //         var visibility = Visibility.Visible;
    //         if (music.Lyrics.TransLyrics == null || music.Lyrics.TransLyrics.Lines.Count == 0)
    //         {
    //             visibility = Visibility.Collapsed;
    //         }
    //
    //         var songLyric = new SongLyric
    //         {
    //             PureLine = music.Lyrics.Lyrics.Lines[i],
    //             Translation = visibility == Visibility.Visible
    //                 ? music.Lyrics.TransLyrics?.Lines[i].CurrentLyric
    //                 : string.Empty,
    //         };
    //         LyricItems.Add(new LyricItemModel(songLyric, i));
    //     }
    //
    //     LyricPosition = 0;
    // }

    /// <summary>
    /// 通过播放进度获取歌词位置
    /// </summary>
    /// <param name="time"></param>
    // private void OnLyricPositionGetter(TimeSpan time)
    // {
    //     if (LyricItems.Count == 0) return;
    //     if (LyricPosition == -1) return;
    //     if (LyricItems.Count <= LyricPosition) return;
    //     if (LyricPosition < LyricItems.Count - 1)
    //     {
    //         if (LyricItems[LyricPosition + 1].SongLyric.PureLine.StartTime < time)
    //         {
    //             LyricPosition++;
    //             OnLyricChanged();
    //         }
    //     }
    // }

    // private void OnLyricChanged()
    // {
    //     if (LyricPosition == -1) return;
    //     if (LyricItems.Count <= LyricPosition) return;
    //     if (LyricPosition < LyricItems.Count - 1 &&
    //         LyricItems[LyricPosition + 1].SongLyric.PureLine is LrcLyricsLine lrcLine)
    //     {
    //         if (lrcLine.StartTime.TotalSeconds - LyricItems[LyricPosition].SongLyric.PureLine.StartTime.TotalSeconds >
    //             1)
    //         {
    //             ChangeLyric();
    //             return;
    //         }
    //     }
    //
    //     ChangeLyric();
    // }

    /// <summary>
    /// 通过播放进度改变歌词
    /// </summary>
    // private void ChangeLyric()
    // {
    //     if (LyricPosition == -1) return;
    //     if (LyricItems.Count <= LyricPosition) return;
    //     if (LyricPosition < LyricItems.Count - 1)
    //     {
    //         if (LyricItems[LyricPosition + 1].SongLyric.PureLine.StartTime < Player.Instance.Position)
    //         {
    //             LyricPosition++;
    //             OnLyricChanged();
    //         }
    //     }
    // }
}