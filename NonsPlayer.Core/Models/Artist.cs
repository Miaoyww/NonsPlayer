using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class Artist : INonsModel
{
    public List<Music> HotMusics;
    public int MusicCount;
    public int MvCount;

    private Artist()
    {
    }

    public static Artist CreatEmpty()
    {
        return new Artist
        {
            Name = "未知艺术家"
        };
    }

    public static async Task<Artist> CreatAsync(long id)
    {
        var result = await Apis.Artist.Detail(id, Nons.Instance);
        return new Artist
        {
            Name = result["data"]["artist"]["name"].ToString(),
            Id = result["data"]["artist"]["id"].ToObject<long>(),
            AvatarUrl = result["data"]["artist"]["avatar"].ToString()
        };
    }

    public static Artist CreatAsync(JObject item)
    {
        return new Artist
        {
            Name = (string) item["name"],
            Id = (int) item["id"],
            AvatarUrl = (string) item["picUrl"],
        };
    }
}