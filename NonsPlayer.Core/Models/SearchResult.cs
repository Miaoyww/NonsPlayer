using System.Security.Cryptography;
using System.Text;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Exceptions;


namespace NonsPlayer.Core.Models;

public class SearchResult: INonsModel
{
    public readonly string KeyWords;
    public SearchResult(string keyWords)
    {
        if (keyWords.Equals(string.Empty)) throw new SearchExceptions("搜索KeyWords不能为空");

        KeyWords = keyWords;
    }

    public IPlaylist[]? Playlists { get; set; }

    public IArtist[]? Artists { get; set; }

    public IMusic[]? Musics { get; set; }

    /// <returns>Md5的b64形式</returns>
    private string GetMd5(string keyWords)
    {
        Md5 = Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(keyWords)));
        return Md5;
    }

    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
}