using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class UserPlaylistBarViewModel
{
    [ObservableProperty] private Visibility playlistVis = Visibility.Collapsed;
    [ObservableProperty] private Visibility tipVis = Visibility.Visible;

    public UserPlaylistBarViewModel()
    {
        UserPlaylistService.Instance.PlaylistUpdated += OnPlaylistUpdated;
    }

    public MusicStateModel MusicStateModel => MusicStateModel.Instance;
    public ObservableCollection<Playlist> CreatedPlaylists => UserPlaylistHelper.Instance.CreatedPlaylists;
    public ObservableCollection<Playlist> SavedPlaylists => UserPlaylistHelper.Instance.SavedPlaylists;


    public void OnPlaylistUpdated()
    {
        UserPlaylistHelper.Instance.CreatedPlaylists.Clear();
        UserPlaylistHelper.Instance.SavedPlaylists.Clear();
        UserPlaylistHelper.Instance.BindData();
        PlaylistVis = Visibility.Visible;
        TipVis = Visibility.Collapsed;
    }
}