using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Services;

public class SavedPlaylistService
{
    public delegate void SavedPlaylistChangedEventHandler();

    public List<long> UserPlaylistIds = new();
    public List<Playlist> CreatedPlaylists = new();
    public List<Playlist> SavedPlaylists = new();

    public static SavedPlaylistService Instance { get; } = new();
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
    }

    public async Task UpdatePlaylists()
    {
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