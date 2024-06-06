using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Models;

public class SearchResult : INonsModel
{
    public readonly string KeyWords;
    public SearchResult(string keyWords)
    {
        if (keyWords.Equals(string.Empty)) throw new SearchExceptions("搜索KeyWords不能为空");

        KeyWords = keyWords;
    }

    public Playlist[]? Playlists { get; set; }

    public Artist[]? Artists { get; set; }

    public Music[]? Musics { get; set; }

    /// <returns>Md5的b64形式</returns>
    private string GetMd5(string keyWords)
    {
        Md5 = MD5.HashData(Encoding.UTF8.GetBytes(keyWords)).ToBase64String();
        return Md5;
    }
}