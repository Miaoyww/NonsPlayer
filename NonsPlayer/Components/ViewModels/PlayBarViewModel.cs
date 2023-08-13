using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class PlayerBarViewModel : ObservableObject
{
    [ObservableProperty] private bool isDragging;
    [ObservableProperty] private TimeSpan lastPosition;
    [ObservableProperty] private TimeSpan newPosition;
    private TimeSpan newPostion;

    public PlayerBarViewModel()
    {
        MusicStateModel.Instance.Volume = double.Parse(RegHelper.Instance.Get(RegHelper.Regs.Volume, 100.0).ToString());
        FavoritePlaylistService.Instance.LikeSongsChanged += UpdateLike;
    }

    public PlayerService PlayerService => PlayerService.Instance;
    public MusicStateModel MusicStateModel => MusicStateModel.Instance;
    
    public void UpdateLike()
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            MusicStateModel.Instance.CurrentSongLiked =
                FavoritePlaylistService.Instance.IsLiked(MusicStateModel.Instance.CurrentMusic.Id);
        });
    }

    public void CurrentTimeSlider_OnValueChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (MusicStateModel.Instance.OnDrag)
        {
            // Player.Instance.Position = TimeSpan.FromSeconds(e.NewValue);
        }
    }

    public void CurrentTimeSlider_PointerEntered(object sender, RoutedEventArgs e)
    {
        MusicStateModel.Instance.OnDrag = true;
        IsDragging = true;
    }

    public void CurrentTimeSlider_PointerExited(object sender, RoutedEventArgs e)
    {
        MusicStateModel.Instance.OnDrag = false;
        IsDragging = false;
    }

    partial void OnIsDraggingChanged(bool value)
    {
        if (value == false)
        {
            // Player.Instance.Position = NewPosition;
        }
    }
}