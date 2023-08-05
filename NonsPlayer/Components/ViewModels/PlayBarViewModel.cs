using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;
using NonsPlayer.Services;

namespace NonsPlayer.Components.ViewModels
{
    public partial class PlayerBarViewModel : ObservableObject
    {
        public PlayerService PlayerService => PlayerService.Instance;
        public MusicState MusicState => MusicState.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;
        private TimeSpan newPostion;

        [ObservableProperty] private bool isDragging;
        [ObservableProperty] private TimeSpan lastPosition;
        [ObservableProperty] private TimeSpan newPosition;

        public PlayerBarViewModel()
        {
            MusicState.Instance.Volume = double.Parse(RegHelper.Instance.Get(RegHelper.Regs.Volume, 0.0).ToString());
            FavoritePlaylistService.Instance.LikeSongsChanged += UpdateLike;
        }

        [RelayCommand]
        public async void Like()
        {
            if (MusicState.Instance.CurrentMusic.IsEmpty)
            {
                return;
            }

            await FavoritePlaylistService.Instance.Like(MusicState.Instance.CurrentMusic.Id);
        }

        public void UpdateLike()
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                MusicState.Instance.CurrentSongLiked =
                    FavoritePlaylistService.Instance.IsLiked(MusicState.Instance.CurrentMusic.Id);
            });
        }

        public void CurrentTimeSlider_OnValueChanged(object sender, RangeBaseValueChangedEventArgs e)
        {
            if (MusicState.Instance.OnDrag)
            {
                // Player.Instance.Position = TimeSpan.FromSeconds(e.NewValue);
            }
        }

        public void CurrentTimeSlider_PointerEntered(object sender, RoutedEventArgs e)
        {
            MusicState.Instance.OnDrag = true;
            IsDragging = true;
        }

        public void CurrentTimeSlider_PointerExited(object sender, RoutedEventArgs e)
        {
            MusicState.Instance.OnDrag = false;
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
}