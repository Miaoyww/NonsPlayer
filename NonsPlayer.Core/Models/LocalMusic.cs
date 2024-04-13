using System.Text.Json.Serialization;
using IF.Lastfm.Core.Api;
using IF.Lastfm.Core.Objects;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Models;

public class LocalMusic : IMusic
{
    [JsonIgnore] public TagLib.File File;
    private const string LastFMAPIKey = "22a44c4543a01040deaf60265a3c30e4";
    private const string LastFMAPISecret = "0a04baf46f4a6f75df9347ba29c488c2";
    private LastfmClient _lastfmClient = new(LastFMAPIKey, LastFMAPISecret);
    public LastAlbum LastAlbum;
    public List<LastArtist> LastArtists;
    public LastTrack LastTrack;

    public LocalMusic(string path)
    {
        File = TagLib.File.Create(path);
        LocalCover = File.Tag.Pictures.Length > 0 ? File.Tag.Pictures[0].Data.Data : null;
        Md5 = File.GetHashCode().ToString();
        Uri = path;
        Album = new Album()
        {
            Name = File.Tag.Album,
            Id = $"{File.Tag.Album}_{Md5}".GetHashCode(),
            AvatarUrl = Uri,
        };
        Artists =
        [
            new()
            {
                Name = File.Tag.FirstPerformer,
                Id = $"{File.Tag.FirstPerformer}_{Md5}".GetHashCode()
            }
        ];
        Duration = File.Properties.Duration;
        Name = File.Tag.Title;
        // Id = $"{Name}_{Md5}".GetHashCode();
    }

    /// <summary>
    /// 从Last.fm获取音乐信息
    /// </summary>
    /// <returns></returns>
    public async Task<bool> TryGetInfo()
    {
        try
        {
            var artistsTasks = Artists.Select(async x => await _lastfmClient.Artist.GetInfoAsync(x.Name)).ToList();
            LastAlbum = (await _lastfmClient.Album.GetInfoAsync(ArtistsName, Artists[0].Name)).Content;
            LastArtists = (await Task.WhenAll(artistsTasks)).Select(x => x.Content).ToList();
            LastTrack = (await _lastfmClient.Track.GetInfoAsync(Name, ArtistsName)).Content;
        }catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
            return false;
        }
       
        return true;
    }
}