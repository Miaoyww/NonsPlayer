using Windows.Storage.Streams;
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
        return GetCacheItem<ImageBrush>(cacheId, async () => new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(url))
        }).Result.Data;
    }

    public static async Task<ImageBrush> GetImageBrushAsync(string cacheId, string url)
    {
        return (await GetCacheItem<ImageBrush>(cacheId, async () => new ImageBrush
        {
            ImageSource = await GetImageStreamFromServer(url)
        })).Data;
    }

    public static async Task<ImageBrush> UpdateImageBrushAsync(string cacheId, string url)
    {
        return (await UpdateCacheItem<ImageBrush>(cacheId, async () => new ImageBrush
        {
            ImageSource = await GetImageStreamFromServer(url)
        })).Data;
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

    public static async Task<Playlist> UpdatePlaylistAsync(string cacheId, string id)
    {
        var playlist = (await UpdateCacheItem<Playlist>(cacheId, async () =>
            await Playlist.CreateAsync(long.Parse(id)))).Data;
        await UpdateImageBrushAsync(playlist.CacheCoverId, playlist.CoverUrl);
        return playlist;
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

    public static async Task<BitmapImage> GetImageStreamFromServer(string imageUrl)
    {
        using (var httpClient = new HttpClient())
        {
            try
            {
                // 发起 HTTP 请求并获取响应数据
                var response = await httpClient.GetAsync(imageUrl);
                response.EnsureSuccessStatusCode();

                // 从响应数据中获取图像的字节流
                byte[] imageBytes = await response.Content.ReadAsByteArrayAsync();

                // 创建 InMemoryRandomAccessStream 并将图像数据写入其中
                using (InMemoryRandomAccessStream stream = new InMemoryRandomAccessStream())
                {
                    using (DataWriter writer = new DataWriter(stream.GetOutputStreamAt(0)))
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
}