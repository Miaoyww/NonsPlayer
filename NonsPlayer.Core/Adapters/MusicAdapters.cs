using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Adapters;

public static class MusicAdapters
{
    public static Music CreateFromPlaylistTrack(JObject item)
    {
        return new Music
        {
            Name = (string) item["name"],
            Id = (int) item["id"],
            AvatarUrl = (string) item["al"]["picUrl"],
            TotalTime = TimeSpan.FromMilliseconds(item["dt"].ToObject<long>()),
            Album = AlbumAdapters.CreateFromPlaylistTrack(item["al"] as JObject),
            Artists = ((JArray) item["ar"]).Select(arItem => ArtistAdapters.CreateFromPlaylistTrack(arItem as JObject)).ToArray()
        };
    }

    public static async Task<Music> CreateById(long id)
    {
        var result = await Apis.Music.Detail(new[] {id}, Nons.Instance);
        return CreateFromPlaylistTrack(result["songs"][0] as JObject);
    }
}