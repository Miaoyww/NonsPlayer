using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using NonsPlayer.Components.Models;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Heplers;

namespace NonsPlayer.Components.ViewModels
{
    public class FunctionBarViewModel : INotifyPropertyChanged
    {

        public GlobalMusicState GlobalMusicState => GlobalMusicState.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;
        public ObservableCollection<UserPlaylistItem> UserPlaylists => UserPlaylistHelper.Instance.UserPlaylists;
        public ObservableCollection<UserPlaylistItem> FavoritePlaylists => UserPlaylistHelper.Instance.SavedPlaylists;


        public FunctionBarViewModel()
        {

        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}