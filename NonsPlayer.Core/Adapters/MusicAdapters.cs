using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Adapters;

public class MusicAdapters: IMusicAdapter
{
    
    public Task<Music> GetMusicAsync(object content)
    {

        throw new NotImplementedException();
    }
    
    public Task<Music[]> GetMusicListAsync(object content)
    {
        throw new NotImplementedException();
    }

    public Music[] GetMusicList(object content)
    {
        var item = content as JArray;
        return item.Select(x => GetMusic(x as JObject)).ToArray();
    }

    public Music GetMusic(object content)
    {
        var item = content as JObject;
        var albumAdapter = new AlbumAdapters();
        return new Music
        {
            Name = (string)item["name"],
            Id = (int)item["id"],
            AvatarUrl = (string)item["al"]["picUrl"],
            Duration = TimeSpan.FromMilliseconds(item["dt"].ToObject<long>()),
            Album =  albumAdapter.GetAlbum(item["al"] as JObject),
            Artists = ((JArray)item["ar"]).Select(arItem => ArtistAdapters.CreateFromPlaylistTrack(arItem as JObject))
                .ToArray(),
            Trans = !((JArray)item["alia"]).Any()
                ? !item.ContainsKey("tns") ? string.Empty : item["tns"][0].ToString()
                : item["alia"][0].ToString(),
            ShareUrl = $"https://music.163.com/song?id={(int)item["id"]}"
        };
    }
    
    public async Task<Music> CreateById(long id)
    {
        var result = await Apis.Music.Detail(new[] { id }, NonsCore.Instance);
        return GetMusic(result["songs"][0] as JObject);
    }

}