using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Adapters;

public class AlbumAdapters: IAlbumAdapter
{
    public Task<Album> GetAlbumAsync(object content)
    {   
        throw new NotImplementedException();
    }

    public Album GetAlbum(object content)
    {
        var item = content as JObject;
        return new Album
        {
            AvatarUrl = item["picUrl"].ToString(),
            Name = item["name"].ToString(),
            Id = item["id"].ToObject<long>(),
            ShareUrl = $"https://music.163.com/album?id={item["id"]}"
        };

    }

}