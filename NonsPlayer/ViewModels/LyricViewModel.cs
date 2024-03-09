using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class LyricViewModel : ObservableRecipient
{
    public ObservableCollection<LyricModel> LyricItems = new();
    [ObservableProperty] private Music currentMusic;
    public static int LyricPosition;

    public PlayerService PlayerService => PlayerService.Instance;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;

    public LyricViewModel()
    {
        Player.Instance.PositionChangedHandle += LyricChanger;
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


            LyricItems.Add(new LyricModel()
            {
                LyricLine = music.Lyrics.Lyrics.Lines[i],
                Translation = visibility == Visibility.Visible
                    ? music.Lyrics.TransLyrics?.Lines[i].CurrentLyric
                    : string.Empty,
                TransVisibility = visibility
            });
        }

        LyricPosition = 0;
    }

    private void LyricChanger(TimeSpan time)
    {
        // if (LyricItems.Count == 0) return;
        // if (LyricPosition >= LyricItems.Count || LyricPosition < 0) LyricPosition = 0;
        // var changed = false;
        // var realPos = Player.Instance.Position;

        // if (LyricItems[LyricPosition].LyricLine.StartTime > realPos) //当感知到进度回溯时执行
        // {
        //     LyricPosition = LyricItems.ToList().FindLastIndex(t => t.LyricLine.StartTime <= realPos) - 1;
        //     if (LyricPosition == -2) LyricPosition = -1;
        //     changed = true;
        // }
        //
        // try
        // {
        //     if (LyricPosition == 0 && LyricItems.Count != 1) changed = false;
        //     while (LyricItems.Count > LyricPosition + 1 &&
        //            LyricItems[LyricPosition + 1].LyricLine.StartTime <= realPos) //正常的滚歌词
        //     {
        //         LyricPosition++;
        //         changed = true;
        //     }
        // }
        // catch
        // {
        //     // ignored
        // }


        // if (changed)
        // {
        // OnLyricChanged();
        // }
    }

    private void OnLyricChanged()
    {
        if (LyricPosition == -1) return;
        if (LyricItems.Count <= LyricPosition) return;
        if (LyricPosition < LyricItems.Count - 1 && LyricItems[LyricPosition + 1].LyricLine is LrcLyricsLine lrcLine)
        {
            if (lrcLine.StartTime.TotalSeconds - LyricItems[LyricPosition].LyricLine.StartTime.TotalSeconds > 1)
            {
                ChangeLyric();
                return;
            }
        }

        ChangeLyric();
    }

    private void ChangeLyric()
    {
        // LyricText = HyPlayList.Lyrics[HyPlayList.LyricPos].LyricLine.CurrentLyric;
        // LyricControl.Lyric = HyPlayList.Lyrics[HyPlayList.LyricPos];
    }
}