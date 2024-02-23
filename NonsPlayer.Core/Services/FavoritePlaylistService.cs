using System.Diagnostics;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Nons.Account;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Core.Services;

public class FavoritePlaylistService
{
    public delegate void LikeSongsChangedEventHandler();

    public string FavoritePlaylistId = string.Empty;
    public bool IsLikeSongsChanged;

    public List<string> LikedSongs = new();
    public static FavoritePlaylistService Instance { get; } = new();

    public event LikeSongsChangedEventHandler LikeSongsChanged;

    public bool IsLiked(long? id)
    {
        if (LikedSongs.Contains(id.ToString())) return true;

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
        if (Account.Instance.IsLoggedIn)
        {
            var likedSongs =
                (JArray)(await Apis.Music.LikeList(FavoritePlaylistId, NonsCore.Instance))["ids"];
            LikedSongs = likedSongs.Select(likedSong => likedSong.ToString()).ToList();
        }
    }

    public async Task<int> LikeAsync(long? id)
    {
        var result = await Apis.Music.Like(id.ToString(), !IsLiked(id), NonsCore.Instance);
        Debug.WriteLine($"喜欢歌曲({id}): {result["code"]}");
        if ((int)result["code"] == 200)
        {
            LikedSongs.Add(id.ToString());
            await UpdatePlaylistInfo().ConfigureAwait(false);
            IsLikeSongsChanged = true;
            LikeSongsChanged();
            return 200;
        }

        return (int)result["code"];
    }
}