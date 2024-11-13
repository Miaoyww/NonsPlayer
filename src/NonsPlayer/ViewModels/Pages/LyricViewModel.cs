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
    
    private async void OnMusicChanged(IMusic music)
    {
        CurrentMusic = music;
        LyricItems.Clear();
        if (music.Lyric != null)
        {
            foreach (var line in music.Lyric.LyricLines)
            {
                LyricItems.Add(line);
            }

            LyricPosition = 0;
        }
        
        if (CurrentMusic is LocalMusic)
        {
            Cover = await ImageHelpers.GetImageBrushAsyncFromBytes(((LocalMusic)CurrentMusic).GetCover());
        }
        else
        {
            Cover =
                Cover = CacheHelper.GetImageBrush(CurrentMusic.CacheAvatarId, CurrentMusic.GetCoverUrl("?param=500x500"));
        }
       
    }
    
}