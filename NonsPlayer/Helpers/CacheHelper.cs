using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Cache;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers;

public static class CacheHelper
{
    public static ImageBrush GetImageBrush(string cacheId, string url)
    {
        var cacheItem = CacheManager.Instance.TryGet<ImageBrush>(cacheId);
        if (cacheItem == null)
        {
            cacheItem = new CacheItem<ImageBrush>
            {
                Data = new ImageBrush
                {
                    ImageSource = new BitmapImage(new Uri(url))
                }
            };
            CacheManager.Instance.Set(cacheId, cacheItem);
        }

        return cacheItem.Data;
    }

    public static async Task<Playlist> GetPlaylistAsync(string cacheId, string id)
    {
        var playlistTemp = CacheManager.Instance.TryGet<Playlist>(cacheId);
        if (playlistTemp == null)
        {
            playlistTemp = new CacheItem<Playlist>
            {
                Data = await Playlist.CreateAsync(long.Parse(id))
            };
        }

        return playlistTemp.Data;
    }
    public static Playlist GetPlaylist(string cacheId, string id)
    {
        var playlistTemp = CacheManager.Instance.TryGet<Playlist>(cacheId);
        if (playlistTemp == null)
        {
            playlistTemp = new CacheItem<Playlist>
            {
                Data = Playlist.CreateAsync(long.Parse(id)).Result
            };
        }

        return playlistTemp.Data;
    }
}