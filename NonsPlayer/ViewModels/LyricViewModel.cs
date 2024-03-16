using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.ViewModels;

public partial class LyricViewModel : ObservableRecipient
{
    public ObservableCollection<LyricItemModel> LyricItems = new();
    [ObservableProperty] private Music currentMusic;
    public int LyricPosition;

    public PlayerService PlayerService => PlayerService.Instance;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;

    public LyricViewModel()
    {
        Player.Instance.PositionChangedHandle += LyricPositionGetter;
        Player.Instance.MusicChangedHandle += OnMusicChanged;
        LyricPosition = 0;
        if (Player.Instance.CurrentMusic == null)
        {
            return;
        }

        OnMusicChanged(Player.Instance.CurrentMusic);
    }

    [RelayCommand]
    public void SwitchPlayMode()
    {
        PlayQueue.Instance.SwitchPlayMode();
    }

    [RelayCommand]
    public void SwitchShuffle()
    {
        PlayQueue.Instance.SwitchShuffle();
    }

    // public Visibility TransVisibility => TranLyric.Equals(string.Empty) ? Visibility.Collapsed : Visibility.Visible;
    private void OnMusicChanged(Music music)
    {
        CurrentMusic = music;
        LyricItems.Clear();

        for (int i = 0; i < music.Lyrics.Lyrics.Lines.Count; i++)
        {
            var visibility = Visibility.Visible;
            if (music.Lyrics.TransLyrics == null || music.Lyrics.TransLyrics.Lines.Count == 0)
            {
                visibility = Visibility.Collapsed;
            }

            var songLyric = new SongLyric
            {
                PureLine = music.Lyrics.Lyrics.Lines[i],
                Translation = visibility == Visibility.Visible
                    ? music.Lyrics.TransLyrics?.Lines[i].CurrentLyric
                    : string.Empty,
            };
            LyricItems.Add(new LyricItemModel(songLyric, i));
        }

        LyricPosition = 0;
    }
    
    /// <summary>
    /// 通过播放进度获取歌词位置
    /// </summary>
    /// <param name="time"></param>
    private void LyricPositionGetter(TimeSpan time)
    {
        if (LyricItems.Count == 0) return;
        if (LyricPosition == -1) return;
        if (LyricItems.Count <= LyricPosition) return;
        if (LyricPosition < LyricItems.Count - 1)
        {
            if (LyricItems[LyricPosition + 1].SongLyric.PureLine.StartTime < time)
            {
                LyricPosition++;
                OnLyricChanged();
            }
        }
    }

    private void OnLyricChanged()
    {
        if (LyricPosition == -1) return;
        if (LyricItems.Count <= LyricPosition) return;
        if (LyricPosition < LyricItems.Count - 1 && LyricItems[LyricPosition + 1].SongLyric.PureLine is LrcLyricsLine lrcLine)
        {
            if (lrcLine.StartTime.TotalSeconds - LyricItems[LyricPosition].SongLyric.PureLine.StartTime.TotalSeconds > 1)
            {
                ChangeLyric();
                return;
            }
        }
        
        ChangeLyric();
    }
    
    /// <summary>
    /// 通过播放进度改变歌词
    /// </summary>
    private void ChangeLyric()
    {
        if (LyricPosition == -1) return;
        if (LyricItems.Count <= LyricPosition) return;
        if (LyricPosition < LyricItems.Count - 1)
        {
            if (LyricItems[LyricPosition + 1].SongLyric.PureLine.StartTime < Player.Instance.Position)
            {
                LyricPosition++;
                OnLyricChanged();
            }
        }
    }
}