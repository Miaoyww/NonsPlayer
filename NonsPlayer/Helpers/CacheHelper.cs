using Windows.Storage.Streams;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers;

public static class CacheHelper
{
    public static async Task<CacheItem<T>> GetCacheItemAsync<T>(string cacheId, Func<Task<T>> createItemAsync)
        where T : class
    {
        try
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
        catch (Exception e)
        {
            Console.WriteLine(e);
            return null;
        }
    }

    public static CacheItem<T> GetCacheItem<T>(string cacheId, Func<T> createItemAsync)
        where T : class
    {
        try
        {
            var cacheItem = CacheManager.Instance.TryGet<T>(cacheId);
            if (cacheItem == null)
            {
                cacheItem = new CacheItem<T>
                {
                    Data = createItemAsync()
                };
                CacheManager.Instance.Set(cacheId, cacheItem);
            }

            return cacheItem;
        }
        catch (Exception e)
        {
            Console.WriteLine(e);
            return null;
        }
    }

    public static async Task<CacheItem<T>> UpdateCacheItem<T>(string cacheId, Func<Task<T>> createItemAsync)
        where T : class

    {
        var cacheItem = new CacheItem<T>
        {
            Data = await createItemAsync()
        };
        CacheManager.Instance.Set(cacheId, cacheItem);
        return cacheItem;
    }

    public static ImageBrush GetImageBrush(string cacheId, string url)
    {
        return GetCacheItemAsync(cacheId, async () => new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(url))
        }).Result.Data;
    }
    public static async Task<ImageBrush> GetImageBrush(string cacheId, byte[] cover)
    { 
        using (var stream = new InMemoryRandomAccessStream())
        {
            // 使用DataWriter将字节数组写入流
            using (DataWriter writer = new DataWriter(stream.GetOutputStreamAt(0)))
            {
                writer.WriteBytes(cover);
                await writer.StoreAsync();
            }

            // 创建BitmapImage并从流中加载图像
            var bitmapImage = new BitmapImage();
            await bitmapImage.SetSourceAsync(stream);
            return GetCacheItemAsync(cacheId, async () => new ImageBrush
            {
                ImageSource = bitmapImage
            }).Result.Data;
        }
        
    }

    public static async Task<ImageBrush> GetImageBrushAsync(string cacheId, string url)
    {
        return (await GetCacheItemAsync(cacheId, async () => new ImageBrush
        {
            ImageSource = await GetBitmapImageFromServer(url)
        })).Data;
    }

    public static async Task<ImageBrush> UpdateImageBrushAsync(string cacheId, string url)
    {
        return (await UpdateCacheItem(cacheId, async () => new ImageBrush
        {
            ImageSource = await GetBitmapImageFromServer(url)
        })).Data;
    }

    public static async Task<Playlist> GetPlaylistAsync(string cacheId, string id)
    {
        return (await GetCacheItemAsync(cacheId, async () =>
            await PlaylistAdaptes.CreateById(long.Parse(id)))).Data;
    }

    public static Playlist GetPlaylistCard(string cacheId, JObject item)
    {
        return GetCacheItem(cacheId, () => PlaylistAdaptes.CreateFromRecommend(item)).Data;
    }

    public static async Task<Playlist> UpdatePlaylistAsync(string cacheId, string id)
    {
        var playlist = (await UpdateCacheItem(cacheId, async () =>
            await PlaylistAdaptes.CreateById(long.Parse(id)))).Data;
        await UpdateImageBrushAsync(playlist.CacheAvatarId, playlist.AvatarUrl);
        return playlist;
    }

    public static Playlist GetPlaylist(string cacheId, string id)
    {
        return GetCacheItemAsync(cacheId, async () =>
            await PlaylistAdaptes.CreateById(long.Parse(id))).Result.Data;
    }

    public static async Task<Music> GetMusicAsync(string cacheId, string id)
    {
        var musicAdapters = new MusicAdapters();
        return (await GetCacheItemAsync(cacheId, async () =>
            await musicAdapters.CreateById(long.Parse(id)))).Data;
    }

    public static Music GetMusic(string cacheId, string id)
    {
        var musicAdapters = new MusicAdapters();
        return GetCacheItemAsync(cacheId, async () =>
            await musicAdapters.CreateById(long.Parse(id))).Result.Data;
    }

    public static async Task<SearchResult> GetSearchResultAsync(string cacheId, string keyWords)
    {
        return (await GetCacheItemAsync(cacheId, async () =>
            await SearchResult.CreateSearchAsync(keyWords))).Data;
    }

    public static SearchResult GetSearchResult(string cacheId, string keyWords)
    {
        return GetCacheItemAsync(cacheId, async () =>
            await SearchResult.CreateSearchAsync(keyWords)).Result.Data;
    }

    public static async Task<BitmapImage> GetBitmapImageFromServer(string imageUrl)
    {
        using (var httpClient = new HttpClient())
        {
            try
            {
                var response = await httpClient.GetAsync(imageUrl);
                response.EnsureSuccessStatusCode();
                var imageBytes = await response.Content.ReadAsByteArrayAsync();
                using (var stream = new InMemoryRandomAccessStream())
                {
                    using (var writer = new DataWriter(stream.GetOutputStreamAt(0)))
                    {
                        writer.WriteBytes(imageBytes);
                        await writer.StoreAsync();
                    }

                    var bitmap = new BitmapImage();
                    await bitmap.SetSourceAsync(stream);
                    return bitmap;
                }
            }
            catch (Exception ex)
            {
                // 处理异常
                Console.WriteLine($"Error: {ex.Message}");
                return null;
            }
        }
    }

    public static async Task<IRandomAccessStream> GetImageStreamFromServer(string imageUrl)
    {
        using var httpClient = new HttpClient();
        try
        {
            var response = await httpClient.GetAsync(imageUrl);
            response.EnsureSuccessStatusCode();
            var imageBytes = await response.Content.ReadAsByteArrayAsync();
            var stream = new InMemoryRandomAccessStream();
            using (var writer = new DataWriter(stream.GetOutputStreamAt(0)))
            {
                writer.WriteBytes(imageBytes);
                await writer.StoreAsync();
            }

            return stream;
        }
        catch (Exception ex)
        {
            // 处理异常
            Console.WriteLine($"Error: {ex.Message}");
            return null;
        }
    }
}