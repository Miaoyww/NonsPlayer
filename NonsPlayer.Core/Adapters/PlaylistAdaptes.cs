using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Adapters;

public static class PlaylistAdaptes
{
    public static Playlist CreateFromUserPlaylist(JObject item)
    {
        return new Playlist
        {
            Id = item["id"].ToObject<long>(),
            Name = item["name"].ToString(),
            AvatarUrl = item["coverImgUrl"].ToString(),
            Creator = item["creator"]["nickname"].ToString(),
            IsCardMode = true
        };
    }

    public static Playlist CreateFromRecommend(JObject item)
    {
        return new Playlist
        {
            Id = item["id"].ToObject<long>(),
            Name = item["name"].ToString(),
            AvatarUrl = item["picUrl"].ToString(),
            ShareUrl = $"https://music.163.com/playlist?id={item["id"]}",
            IsCardMode = true
        };
    }

    public static async Task<Playlist> CreateById(long id)
    {
        var playlist = new Playlist();
        await playlist.LoadAsync(id);
        return playlist;
    }

    public static Playlist CreateFromSearch(JObject content)
    {
        return new Playlist
        {
            Id = content["id"].ToObject<long>(),
            Name = content["name"].ToString(),
            AvatarUrl = content["coverImgUrl"].ToString(),
            Creator = content["creator"].ToString(),
            ShareUrl = $"https://music.163.com/playlist?id={content["id"]}",
            IsCardMode = true
        };
    }
}