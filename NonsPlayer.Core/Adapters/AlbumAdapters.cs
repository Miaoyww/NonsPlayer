using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Adapters;

public static class AlbumAdapters
{
    public static Album CreateFromPlaylistTrack(JObject item)
    {
        return new Album
        {
            AvatarUrl = item["picUrl"].ToString(),
            Name = item["name"].ToString(),
            Id = item["id"].ToObject<long>()
        };
    }
}