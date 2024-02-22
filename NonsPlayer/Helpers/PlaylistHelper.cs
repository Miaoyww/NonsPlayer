using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Core.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Helpers;

public static class PlaylistHelper
{
    public static void OpenMusicListDetail(long id, INavigationService navigationService)
    {
        navigationService.NavigateTo(typeof(PlaylistDetailViewModel).FullName!, id);
    }
}

[INotifyPropertyChanged]
public partial class UserPlaylistHelper
{
    public static UserPlaylistHelper Instance { get; } = new();

    public ObservableCollection<Playlist> CreatedPlaylists { get; } = new();

    public ObservableCollection<Playlist> SavedPlaylists { get; } = new();

    public async void Init()
    {
        var test = await Apis.User.Playlist(Account.Instance.Uid, NonsCore.Instance);
        var result = (JArray)test["playlist"];
        UserPlaylistService.Instance.Init(result);
        foreach (var playlistItem in result)
            if (playlistItem["name"].ToString() == Account.Instance.Name + "喜欢的音乐")
            {
                FavoritePlaylistService.Instance.Init(playlistItem["id"].ToString());
                break;
            }
    }


    public void BindData()
    {
        foreach (var item in UserPlaylistService.Instance.CreatedPlaylists)
            ServiceHelper.DispatcherQueue.TryEnqueue(() => { CreatedPlaylists.Add(item); });

        foreach (var item in UserPlaylistService.Instance.SavedPlaylists)
            ServiceHelper.DispatcherQueue.TryEnqueue(() => { SavedPlaylists.Add(item); });
    }
}