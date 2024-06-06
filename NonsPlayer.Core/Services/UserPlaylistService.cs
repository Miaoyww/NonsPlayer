using System.Diagnostics;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Nons.Account;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Services;

public class UserPlaylistService
{
    public delegate void PlaylistUpdatedEventHandler();

    public delegate void SavedPlaylistChangedEventHandler();

    public List<Playlist> CreatedPlaylists = new();
    public List<Playlist> SavedPlaylists = new();

    public List<long> UserPlaylistIds = new();
    public static UserPlaylistService Instance { get; } = new();

    public event PlaylistUpdatedEventHandler PlaylistUpdated;
    public event SavedPlaylistChangedEventHandler SavedPlaylistChanged;

    public bool IsLiked(long id)
    {
        if (UserPlaylistIds.Contains(id)) return true;

        return false;
    }

    public void Init(JArray value)
    {
        foreach (var item in value) ParseInfo(item as JObject);
        PlaylistUpdated.Invoke();
        var timer = new Timer();
        timer.Interval = 1000 * 30;
        timer.Elapsed += async (sender, args) => await UpdatePlaylists().ConfigureAwait(false);
    }


    public async Task Like(long id)
    {
        if (CreatedPlaylists.Select(x => x.Id).Contains(id)) return;
        var result = await Apis.Playlist.Subscribe(id, !IsLiked(id), NonsCore.Instance);
        if (result["code"].ToObject<int>() == 200) await UpdatePlaylists();
    }

    public async Task UpdatePlaylists()
    {
        try
        {
            var result = (JArray)(await Apis.User.Playlist(Account.Instance.Uid, NonsCore.Instance))["playlist"];
            CreatedPlaylists.Clear();
            SavedPlaylists.Clear();
            UserPlaylistIds.Clear();
            foreach (var item in result) ParseInfo(item as JObject);
            PlaylistUpdated.Invoke();
        }
        catch (Exception e)
        {
            Debug.WriteLine(e);
        }
    }

    private void ParseInfo(JObject item)
    {
        // var playlist = PlaylistAdaptes.CreateFromUserPlaylist(item);
        // if (playlist.Creator.Equals(Account.Instance.Name))
        //     CreatedPlaylists.Add(playlist);
        //
        // else
        //     SavedPlaylists.Add(playlist);
        //
        // UserPlaylistIds.Add(playlist.Id);
    }
}