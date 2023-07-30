using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using NonsPlayer.Components.Models;
using NonsPlayer.Heplers;
using NonsPlayer.Models;

namespace NonsPlayer.Components.ViewModels
{
    public class FunctionBarViewModel
    {
        public MusicState MusicState => MusicState.Instance;
        public ObservableCollection<UserPlaylistItem> UserPlaylists => UserPlaylistHelper.Instance.UserPlaylists;
    }
}