using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Views.Pages;
using static Vanara.PInvoke.Ole32.PROPERTYKEY.System;

namespace NonsPlayer.ViewModels;

public partial class LyricViewModel : ObservableRecipient
{
    public ObservableCollection<LyricLine> LyricItems = new();
    [ObservableProperty] private IMusic currentMusic;
    public static int LyricPosition;

    public PlayerService PlayerService => PlayerService.Instance;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;
    [ObservableProperty] private ImageBrush cover;
    public LyricViewModel()
    {
        Player.Instance.PositionChanged += LyricChanger;
        Player.Instance.MusicChanged += OnMusicChanged;
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
    private void OnMusicChanged(IMusic music)
    {
        CurrentMusic = music;
        LyricItems.Clear();

        foreach (var line in music.Lyric.LyricLines)
        {
            LyricItems.Add(line);
        }

        LyricPosition = 0;
        Cover = CacheHelper.GetImageBrush(CurrentMusic.CacheAvatarId, CurrentMusic.GetCoverUrl("?param=500x500"));
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
        ChangeLyric();
    }

    private void ChangeLyric()
    {
        // LyricText = HyPlayList.Lyrics[HyPlayList.LyricPos].LyricLine.CurrentLyric;
        // LyricControl.Lyric = HyPlayList.Lyrics[HyPlayList.LyricPos];
    }
}