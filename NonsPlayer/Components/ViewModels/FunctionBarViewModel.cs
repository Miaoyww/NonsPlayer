using System.Collections.ObjectModel;
using NonsPlayer.Core.Models;
using NonsPlayer.Heplers;
using NonsPlayer.Models;

namespace NonsPlayer.Components.ViewModels;

public class FunctionBarViewModel
{
    public MusicState MusicState => MusicState.Instance;
    public ObservableCollection<Playlist> UserPlaylists => UserPlaylistHelper.Instance.CreatedPlaylists;
    public ObservableCollection<Playlist> SavedPlaylists => UserPlaylistHelper.Instance.SavedPlaylists;
}