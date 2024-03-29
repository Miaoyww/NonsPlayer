﻿using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Adapters;

public static class ArtistAdapters
{
    public static async Task<Artist> CreateById(long id)
    {
        var result = await Apis.Artist.Detail(id, NonsCore.Instance);
        return new Artist
        {
            Id = result["data"]["artist"]["id"].ToObject<long>(),
            Name = result["data"]["artist"]["name"].ToString(),
            Description = result["data"]["artist"]["briefDesc"].ToString(),
            AvatarUrl = result["data"]["artist"]["avatar"].ToString()
        };
    }

    public static Artist CreateFromSearch(JObject content)
    {
        return new Artist
        {
            Id = content["id"].ToObject<long>(),
            Name = content["name"].ToString(),
            Trans = content["trans"].ToString(),
            AvatarUrl = content["picUrl"].ToString()
        };
    }

    public static Artist CreateFromPlaylistTrack(JObject item)
    {
        return new Artist
        {
            Id = item["id"].ToObject<long>(),
            Name = item["name"].ToString()
        };
    }
}