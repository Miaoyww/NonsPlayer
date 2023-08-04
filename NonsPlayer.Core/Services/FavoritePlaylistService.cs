using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Services;

public class FavoritePlaylistService
{
    public static FavoritePlaylistService Instance { get; } = new();

    public List<string> LikedSongs = new();
    public string FavoritePlaylistId = string.Empty;
    public bool IsLikeSongsChanged = false;

    public delegate void LikeSongsChangedEventHandler();

    public event LikeSongsChangedEventHandler LikeSongsChanged;

    public bool IsLiked(long? id)
    {
        if (LikedSongs.Contains(id.ToString()))
        {
            return true;
        }

        return false;
    }

    public async void Init(string id)
    {
        FavoritePlaylistId = id;
        await UpdatePlaylistInfo().ConfigureAwait(false);
        var timer = new Timer();
        timer.Interval = 1000 * 30;
        timer.Elapsed += async (sender, args) => await UpdatePlaylistInfo().ConfigureAwait(false);
    }

    public async Task UpdatePlaylistInfo()
    {
        var likedSongs =
            (JArray) (await Apis.Music.LikeList(FavoritePlaylistId, Nons.Instance))["ids"];
        LikedSongs = likedSongs.Select(likedSong => likedSong.ToString()).ToList();
    }

    public async Task<bool> Like(long? id)
    {
        var result = await Apis.Music.Like(id.ToString(), !IsLiked(id), Nons.Instance);
        Debug.WriteLine($"喜欢歌曲({id}): {result["code"]}");
        LikedSongs.Add(id.ToString());
        await UpdatePlaylistInfo().ConfigureAwait(false);
        if ((int) result["code"] == 200)
        {
            IsLikeSongsChanged = true;
            LikeSongsChanged();
            return true;
        }

        return false;
    }
}