using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Models;

public class SearchResult : INonsModel
{
    public readonly string KeyWords;
    private MusicAdapters _musicAdapters = new MusicAdapters();
    private SearchResult(string keyWords)
    {
        if (keyWords.Equals(string.Empty)) throw new SearchExceptions("搜索KeyWords不能为空");

        KeyWords = keyWords;
    }

    public Playlist[] Playlists { get; set; }

    public Artist[] Artists { get; set; }

    public Music[] Musics { get; set; }

    public static async Task<SearchResult> CreateSearchAsync(string keyWords)
    {
        var i = new SearchResult(keyWords);
        Debug.WriteLine(i.GetMd5(keyWords));
        return i;
    }

    public async Task<Music[]> SearchMusics(int limit = 20)
    {
        var result = await Apis.Search.Default(KeyWords, limit, 1, NonsCore.Instance);
        var tasks = ((JArray)result["result"]["songs"])
            .Select(x => _musicAdapters.CreateById(x["id"].ToObject<long>())).ToList();
        Musics = await Task.WhenAll(tasks);
        return Musics;
    }

    public async Task<Artist[]> SearchArtists(int limit = 2)
    {
        var result = await Apis.Search.Default(KeyWords, limit, 100, NonsCore.Instance);
        Artists = ((JArray)result["result"]["artists"])
            .Select(x => ArtistAdapters.CreateFromSearch((JObject)x)).ToArray();
        return Artists;
    }

    public async Task<Playlist[]> SearchPlaylists(int limit = 2)
    {
        var result = await Apis.Search.Default(KeyWords, limit, 1000, NonsCore.Instance);
        var tasks = ((JArray)result["result"]["playlists"])
            .Select(x => PlaylistAdaptes.CreateById(x["id"].ToObject<long>())).ToList();
        Playlists = await Task.WhenAll(tasks);
        return Playlists;
    }

    /// <returns>Md5的b64形式</returns>
    private string GetMd5(string keyWords)
    {
        Md5 = MD5.HashData(Encoding.UTF8.GetBytes(keyWords)).ToBase64String();
        return Md5;
    }
}