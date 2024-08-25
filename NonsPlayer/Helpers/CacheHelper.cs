using Microsoft.Extensions.Logging;
using Windows.Storage.Streams;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Utils;
using SharpCompress.Common;

namespace NonsPlayer.Helpers;

public static class CacheHelper
{
    public static CacheService Service = App.GetService<CacheService>();

    public static async Task<T> GetCacheItemAsync<T>(string cacheId, Func<Task<T>>? createItemAsync = null)
    {
        try
        {
            if (!Service.TryGet<T>(cacheId, out var result))
            {
                if (createItemAsync != null)
                {
                    result = await createItemAsync();
                }

                Service.AddOrUpdate(cacheId, result);
                return result;
            }

            return result;
        }
        catch (Exception e)
        {
            Service.Logger.LogError($"Exception threw while getting cache. cache id:{cacheId} \n{e}");
            return default;
        }
    }

    public static T GetCacheItem<T>(string cacheId, Func<T>? createItem = null)
    {
        try
        {
            if (!Service.TryGet<T>(cacheId, out var result))
            {
                if (createItem != null)
                {
                    result = createItem();
                }

                Service.AddOrUpdate(cacheId, result);
            }

            return result;
        }
        catch (Exception e)
        {
            Service.Logger.LogError($"Exception threw while getting cache. cache id:{cacheId} \n{e}");
            return default;
        }
    }

    public static ImageBrush GetImageBrush(string cacheId, string url)
    {
        return
            GetCacheItem(
                cacheId,
                () => new ImageBrush { ImageSource = new BitmapImage(new Uri(url)) });
    }

    public static async Task<ImageBrush?> GetImageBrushAsync(string cacheId, byte[] cover)
    {
        return await GetCacheItemAsync(cacheId, () => ImageUtils.GetImageBrushAsync(cover));
    }

    public static async Task<ImageBrush> GetImageBrushAsync(string cacheId, string url)
    {
        return (await GetCacheItemAsync(cacheId,
            async () => new ImageBrush { ImageSource = await ImageUtils.GetBitmapImageFromServer(url) }));
    }

    public static async Task<IPlaylist?> GetPlaylistAsync(string cacheId, Func<Task<IPlaylist>> method)
    {
        return (await GetCacheItemAsync<IPlaylist>(cacheId, method));
    }

    public static IPlaylist? GetPlaylist(string cacheId, Func<IPlaylist> method)
    {
        return (GetCacheItem(cacheId, method));
    }
    //
    // public static Music GetPlaylistCard(string cacheId, JObject item)
    // {
    //     return GetCacheItem(cacheId, () => PlaylistAdaptes.CreateFromRecommend(item)).Value;
    // }
    //
    // public static async Task<Music> UpdatePlaylistAsync(string cacheId, string id)
    // {
    //     var playlist = (await UpdateCacheItem(cacheId, async () =>
    //         await PlaylistAdaptes.CreateById(long.Parse(id)))).Value;
    //     await UpdateImageBrushAsync(playlist.CacheAvatarId, playlist.AvatarUrl);
    //     return playlist;
    // }
    //
    // public static Music GetPlaylist(string cacheId, string id)
    // {
    //     return GetCacheItemAsync(cacheId, async () =>
    //         await PlaylistAdaptes.CreateById(long.Parse(id))).Result.Value;
    // }
    //
    // public static async Task<Music> GetMusicAsync(string cacheId, string id)
    // {
    //     var musicAdapters = new MusicAdapters();
    //     return (await GetCacheItemAsync(cacheId, async () =>
    //         await musicAdapters.CreateById(long.Parse(id)))).Value;
    // }

    public static IMusic GetMusic(string cacheId, string id, string platform)
    {
        var musicAdapters = AdapterService.Instance.GetAdapter(platform).Music;
        return GetCacheItemAsync(cacheId, async () =>
            await musicAdapters.GetMusicAsync(long.Parse(id))).Result;
    }

    // public static async Task<SearchResult> GetSearchResultAsync(string cacheId, string keyWords)
    // {
    //     return (await GetCacheItemAsync(cacheId, async () =>
    //         await SearchResult.CreateSearchAsync(keyWords))).Value;
    // }
    //
    public static List<SearchResult>? GetSearchResult(string cacheId)
    {
        return GetCacheItemAsync<List<SearchResult>>(cacheId, null).Result;
    }
}