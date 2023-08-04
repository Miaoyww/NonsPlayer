using System.Diagnostics;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Services;

public class SavedPlaylistService
{
    public static SavedPlaylistService Instance { get; } = new();
    public List<string> SavedPlaylists = new();

    public delegate void SavedPlaylistChangedEventHandler();

    public event SavedPlaylistChangedEventHandler SavedPlaylistChanged;

    public bool IsLiked(long? id)
    {
        if (SavedPlaylists.Contains(id.ToString()))
        {
            return true;
        }

        return false;
    }

    public void Init(List<string> savedPlaylistIds)
    {
        SavedPlaylists = savedPlaylistIds;
    }
}