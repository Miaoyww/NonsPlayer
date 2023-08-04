using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;
using NonsPlayer.Services;

namespace NonsPlayer.Components.ViewModels
{
    public partial class PlayerBarViewModel
    {
        public PlayerService PlayerService => PlayerService.Instance;
        public MusicState MusicState => MusicState.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;

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
    }
}