using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Heplers;
using NonsPlayer.Models;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class FunctionBarViewModel
{
    public MusicStateViewModel MusicStateViewModel => MusicStateViewModel.Instance;
    public ObservableCollection<Playlist> CreatedPlaylists => UserPlaylistHelper.Instance.CreatedPlaylists;
    public ObservableCollection<Playlist> SavedPlaylists => UserPlaylistHelper.Instance.SavedPlaylists;

    public FunctionBarViewModel()
    {
        UserPlaylistService.Instance.PlaylistUpdated += OnPlaylistUpdated;
    }


    public void OnPlaylistUpdated()
    {
        UserPlaylistHelper.Instance.CreatedPlaylists.Clear();
        UserPlaylistHelper.Instance.SavedPlaylists.Clear();
        UserPlaylistHelper.Instance.BindData();
    }
}