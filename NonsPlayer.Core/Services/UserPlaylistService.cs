using System.Diagnostics;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Services;

public class UserPlaylistService
{
    public delegate void SavedPlaylistChangedEventHandler();

    public List<long> UserPlaylistIds = new();
    public List<Playlist> CreatedPlaylists = new();
    public List<Playlist> SavedPlaylists = new();

    public static UserPlaylistService Instance { get; } = new();
    public event SavedPlaylistChangedEventHandler SavedPlaylistChanged;

    public bool IsLiked(long id)
    {
        if (UserPlaylistIds.Contains(id)) return true;

        return false;
    }

    public void Init(JArray value)
    {
        foreach (var item in value)
        {
            ParseInfo(item as JObject);
        }

        var timer = new Timer();
        timer.Interval = 1000 * 30;
        timer.Elapsed += async (sender, args) => await UpdatePlaylists().ConfigureAwait(false);
    }

    public delegate void PlaylistUpdatedEventHandler();

    public event PlaylistUpdatedEventHandler PlaylistUpdated;

    public async Task Like(long id)
    {
        var result = await Apis.Playlist.Subscribe(id, !IsLiked(id), Nons.Instance);
        if (result["code"].ToObject<int>() == 200)
        {
            await UpdatePlaylists();
        }
        
    }

    public async Task UpdatePlaylists()
    {
        try
        {
            var result = (JArray) (await Apis.User.Playlist(Account.Account.Instance.Uid, Nons.Instance))["playlist"];
            CreatedPlaylists.Clear();
            SavedPlaylists.Clear();
            UserPlaylistIds.Clear();
            foreach (var item in result)
            {
                ParseInfo(item as JObject);
            }

            PlaylistUpdated();
        }
        catch (Exception e)
        {
            Debug.WriteLine(e);
        }
    }

    private void ParseInfo(JObject item)
    {
        var playlist = PlaylistAdaptes.CreateFromUserPlaylist(item);
        if (playlist.Creator.Equals(Account.Account.Instance.Name))
        {
            CreatedPlaylists.Add(playlist);
        }

        else
        {
            SavedPlaylists.Add(playlist);
        }

        UserPlaylistIds.Add(playlist.Id);
    }
}