using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Models;
using NonsPlayer.Services;

namespace NonsPlayer.Components.ViewModels
{
    public class PlayerBarViewModel
    {
        public PlayerService PlayerService => PlayerService.Instance;
        public MusicState MusicState => MusicState.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;

        public PlayerBarViewModel()
        {
            MusicState.Instance.Volume = double.Parse(RegHelper.Instance.Get(RegHelper.Regs.Volume, 0.0).ToString());
        }
    }
}