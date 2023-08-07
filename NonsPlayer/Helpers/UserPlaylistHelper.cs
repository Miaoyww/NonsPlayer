using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Heplers;

[INotifyPropertyChanged]
public partial class UserPlaylistHelper
{
    public static UserPlaylistHelper Instance { get; } = new();

    public ObservableCollection<Playlist> UserPlaylists { get; } = new();

    public ObservableCollection<Playlist> SavedPlaylists { get; } = new();

    public async void Init()
    {
        var result = (JArray) (await Apis.User.Playlist(Account.Instance.Uid, Nons.Instance))["playlist"];
        // var savedPlaylistTasks = new List<Task<Playlist>>();
        var userPlaylistTasks = new List<Task<Playlist>>();
        var savedPlaylistIds = new List<string>();
        foreach (var playlistItem in result)
        {
            savedPlaylistIds.Add(playlistItem["id"].ToString());
            if (playlistItem["name"].ToString() == Account.Instance.Name + "喜欢的音乐")
                FavoritePlaylistService.Instance.Init(playlistItem["id"].ToString());

            var playlistTask = CacheHelper.GetPlaylistCardAsync(
                playlistItem["id"] + "_playlist", new JObject
                {
                    new JProperty("id", playlistItem["id"]),
                    new JProperty("name", playlistItem["name"]),
                    new JProperty("picUrl", playlistItem["coverImgUrl"])
                });
            if ((bool) playlistItem["subscribed"])
            {
                // savedPlaylistTasks.Add(playlistTask);
            }
            else
            {
                userPlaylistTasks.Add(playlistTask);
            }
        }

        SavedPlaylistService.Instance.Init(savedPlaylistIds);
        var whenAllResult = await Task
            .WhenAll(userPlaylistTasks)
            .ConfigureAwait(false);
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            // whenAllResult.ToList().ForEach(item => SavedPlaylists.Add(item));
            whenAllResult.ToList().ForEach(item => UserPlaylists.Add(item));
        });
    }
}