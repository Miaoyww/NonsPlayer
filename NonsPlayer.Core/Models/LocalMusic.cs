using System.Text;
using System.Text.Json.Serialization;
using IF.Lastfm.Core.Objects;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Models;

public class LocalMusic : IMusic
{
    [JsonIgnore] public TagLib.File? File;
    public string FilePath;
    public bool IsInit = false;
    public LastAlbum? LastAlbum;
    public List<LastArtist>? LastArtists;
    public LastTrack? LastTrack;

    public LocalMusic(string path)
    {
        FilePath = path;
    }

    public LocalMusic(TagLib.File file)
    {
        File = file;
        FilePath = file.Name;
    }
    public void ReadMp3Tags(string filePath)
    {
    }
    public void Init()
    {
        IsInit = true;
        if (File == null) File = TagLib.File.Create(FilePath);
        LocalCover = File.Tag.Pictures.Length > 0 ? File.Tag.Pictures[0].Data.Data : null;
        Name = File.Tag.Title;
        if (string.IsNullOrEmpty(Name))
        {
            Name = Path.GetFileNameWithoutExtension(File.Name);
        }

        Md5 = File.GetHashCode().ToString();
        Id = $"{Name}_{Md5}";
        Url = File.Name;
        Album = new LocalAlbum() { Name = File.Tag.Album, Id = $"{File.Tag.Album}_{Md5}", AvatarUrl = Url, };
        Artists =
        [
            new LocalArtist() { Name = File.Tag.FirstPerformer, Id = $"{File.Tag.FirstPerformer}_{Md5}" }
        ];
        Duration = File.Properties.Duration;
    }

    /// <summary>
    /// 从Last.fm获取音乐信息
    /// </summary>
    /// <returns></returns>
    public async Task<bool> TryGetInfo()
    {
        try
        {
            var artistsTasks = Artists.Select(async x => await LastFMService.Instance.LastfmClient.Artist.GetInfoAsync(x.Name)).ToList();
            LastAlbum = (await LastFMService.Instance.LastfmClient.Album.GetInfoAsync(Artists.ToString(), Artists[0].Name)).Content;
            LastArtists = (await Task.WhenAll(artistsTasks)).Select(x => x.Content).ToList();
            LastTrack = (await LastFMService.Instance.LastfmClient.Track.GetInfoAsync(Name, Artists.ToString())).Content;
        }
        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
            return false;
        }

        return true;
        return true;
    }

    private string ConvertEncoding(string str, Encoding from, Encoding to)
    {
        byte[] fromBytes = from.GetBytes(str);
        byte[] toBytes = Encoding.Convert(from, to, fromBytes);
        return to.GetString(toBytes);
    }

    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
    public IAlbum Album { get; set; }
    public IArtist[] Artists { get; set; }
    public bool IsEmpty { get; set; }
    public TimeSpan Duration { get; set; }
    public string Url { get; set; }
    public Lyric Lyric { get; set; }
    public byte[]? LocalCover { get; set; }
    public IAdapter Adapter { get; set; }
    public bool IsLiked { get; set; }
    public MusicQualityLevel[] QualityLevels { get; set; }
    public string? Trans { get; set; }

    public Task<string> GetUrl(MusicQualityLevel quality = MusicQualityLevel.Standard)
    {
        return Task.FromResult(Url);
    }

    public Task<Lyric?> GetLyric()
    {
        return null;
    }

    public string GetCoverUrl(string param = "")
    {
        return string.Empty;
    }

    public Task<bool> Like(bool like)
    {
        return Task.FromResult(false);
    }

    public Task<bool> GetLikeState()
    {
        return Task.FromResult(false);
    }
}