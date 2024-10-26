using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class PlayerBarViewModel : ObservableObject
{
    [ObservableProperty] private bool isDragging;
    [ObservableProperty] private TimeSpan lastPosition;
    [ObservableProperty] private TimeSpan newPosition;
    [ObservableProperty] private Visibility infoVisibility;
    [ObservableProperty] private Visibility artistVisibility = Visibility.Collapsed;
    private TimeSpan newPostion;

    public PlayerBarViewModel()
    {
        PlayerService.CurrentMusicChanged += MusicChangedHandle;
        ChangeVisibility(null);
    }

    private async void MusicChangedHandle(IMusic currentmusic)
    {
        var state = await currentmusic.GetLikeState();
        MusicStateModel.Instance.CurrentSongLiked = state;
        ChangeVisibility(currentmusic);
    }

    private void ChangeVisibility(IMusic currentmusic)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            InfoVisibility = currentmusic == null ? Visibility.Collapsed : Visibility.Visible;
            if (currentmusic!=null)
            {
                ArtistVisibility = Visibility.Visible;
                if (currentmusic.Artists == null) ArtistVisibility = Visibility.Collapsed;
                if (currentmusic.Artists.Length == 0) ArtistVisibility = Visibility.Collapsed;
                if (string.IsNullOrEmpty(currentmusic.Artists[0].Name)) ArtistVisibility = Visibility.Collapsed;
            }

        }); 
    }
    public PlayerService PlayerService => PlayerService.Instance;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;
    
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
}