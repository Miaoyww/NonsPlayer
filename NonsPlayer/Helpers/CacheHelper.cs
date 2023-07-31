using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers;

public static class CacheHelper
{
    public static async Task<CacheItem<T>> GetCacheItem<T>(string cacheId, Func<Task<T>> createItemAsync)
        where T : class
    {
        var cacheItem = CacheManager.Instance.TryGet<T>(cacheId);
        if (cacheItem == null)
        {
            cacheItem = new CacheItem<T>
            {
                Data = await createItemAsync()
            };
            CacheManager.Instance.Set(cacheId, cacheItem);
        }

        return cacheItem;
    }

    public static ImageBrush GetImageBrush(string cacheId, string url)
    {
        return GetCacheItem<ImageBrush>(cacheId, async () => new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(url))
        }).Result.Data;
    }

    public static async Task<Playlist> GetPlaylistAsync(string cacheId, string id)
    {
        return (await GetCacheItem<Playlist>(cacheId, async () =>
            await Playlist.CreateAsync(long.Parse(id)))).Data;
    }

    public static async Task<Playlist> GetPlaylistCardAsync(string cacheId, JObject item)
    {
        return (await GetCacheItem<Playlist>(cacheId, async () =>
            await Playlist.CreateCardModeAsync(item))).Data;
    }

    public static Playlist GetPlaylist(string cacheId, string id)
    {
        return GetCacheItem<Playlist>(cacheId, async () =>
            await Playlist.CreateAsync(long.Parse(id))).Result.Data;
    }

    public static async Task<Music> GetMusicAsync(string cacheId, string id)
    {
        return (await GetCacheItem<Music>(cacheId, async () =>
            await Music.CreateAsync(long.Parse(id)))).Data;
    }

    public static Music GetMusic(string cacheId, string id)
    {
        return GetCacheItem<Music>(cacheId, async () =>
            await Music.CreateAsync(long.Parse(id))).Result.Data;
    }
}