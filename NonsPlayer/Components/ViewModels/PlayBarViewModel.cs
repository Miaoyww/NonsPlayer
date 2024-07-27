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
    private TimeSpan newPostion;

    public PlayerBarViewModel()
    {
        Player.Instance.MusicChangedHandle += MusicChangedHandle;
        ChangeVisibility(null);
    }

    private void MusicChangedHandle(IMusic currentmusic)
    {
        ChangeVisibility(currentmusic);
    }

    private void ChangeVisibility(IMusic currentmusic)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            InfoVisibility = currentmusic == null ? Visibility.Collapsed : Visibility.Visible;
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