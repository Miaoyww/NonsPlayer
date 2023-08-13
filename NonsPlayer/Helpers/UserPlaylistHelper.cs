using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Heplers;

[INotifyPropertyChanged]
public partial class UserPlaylistHelper
{
    public static UserPlaylistHelper Instance { get; } = new();

    public ObservableCollection<PlaylistModel> CreatedPlaylists { get; } = new();

    public ObservableCollection<PlaylistModel> SavedPlaylists { get; } = new();

    public async void Init()
    {
        var result = (JArray) (await Apis.User.Playlist(Account.Instance.Uid, Nons.Instance))["playlist"];
        UserPlaylistService.Instance.Init(result);
        foreach (var playlistItem in result)
        {
            if (playlistItem["name"].ToString() == Account.Instance.Name + "喜欢的音乐")
            {
                FavoritePlaylistService.Instance.Init(playlistItem["id"].ToString());
                break;
            }
        }
        BindData();
    }

    
    public void BindData()
    {
        foreach (var item in UserPlaylistService.Instance.CreatedPlaylists)
        {
            CreatedPlaylists.Add(item);
        }

        foreach (var item in UserPlaylistService.Instance.SavedPlaylists)
        {
            SavedPlaylists.Add(item);
        }
    }
}