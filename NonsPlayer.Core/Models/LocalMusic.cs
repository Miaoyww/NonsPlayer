using ATL;
using System.Text;
using System.Text.Json.Serialization;
using IF.Lastfm.Core.Objects;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Services;
using NonsPlayer.Core.Utils;
using SixLabors.ImageSharp;
using SixLabors.ImageSharp.Formats.Jpeg;
using SixLabors.ImageSharp.Processing;

namespace NonsPlayer.Core.Models;

public class LocalMusic : IMusic
{
    public readonly string FilePath;
    public bool IsInit = false;
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
    public bool Available { get; set; }
    public byte[]? Cover { get; set; }
    public IAdapter Adapter { get; set; }
    public bool IsLiked { get; set; }
    public MusicQualityLevel[] QualityLevels { get; set; }
    public string? Trans { get; set; }

    public LastAlbum? LastAlbum;
    public List<LastArtist>? LastArtists;
    public LastTrack? LastTrack;

    public LocalMusic(string path)
    {
        Available = true;
        FilePath = path;
    }

    public void Init()
    {
        if (IsInit) return;
        IsInit = true;
        var track = new Track(FilePath);

        Name = track.Title;
        if (string.IsNullOrEmpty(Name))
        {
            Name = Path.GetFileNameWithoutExtension(track.Title);
        }

        Cover = LocalUtils.CompressAndConvertToByteArray(GetCover(track), 80, 80);
        Md5 = track.GetHashCode().ToString();
        Id = $"{Name}_{Md5}";
        Url = track.Path;
        Album = new LocalAlbum() { Name = track.Album, Id = $"{track.Album}_{Md5}", AvatarUrl = Url, Songs = [this] };

        var artists = track
            .Artist
            .Split(ConfigManager.Instance.Settings.LocalArtistSep.ToArray(), StringSplitOptions.None).ToList();
        Artists = new IArtist[artists.Count];
        for (int i = 0; i < artists.Count; i++)
        {
            Artists[i] = new LocalArtist{ 
                Name = artists[i], 
                Id = $"{artists[i]}_{Md5}", 
                Songs = [this]
            };
        }
        Duration = TimeSpan.FromSeconds(track.Duration);
    }

    public byte[] GetCover()
    {
        var track = new Track(FilePath);
        foreach (PictureInfo pic in track.EmbeddedPictures)
        {
            return pic.PictureData;
        }

        return null;
    }

    public byte[] GetCover(Track track)
    {
        foreach (PictureInfo pic in track.EmbeddedPictures)
        {
            return pic.PictureData;
        }

        return null;
    }

    /// <summary>
    /// 从Last.fm获取音乐信息
    /// </summary>
    /// <returns></returns>
    public async Task<bool> TryGetInfo()
    {
        try
        {
            var artistsTasks = Artists
                .Select(async x => await LastFMService.Instance.LastfmClient.Artist.GetInfoAsync(x.Name)).ToList();
            LastAlbum = (await LastFMService.Instance.LastfmClient.Album.GetInfoAsync(Artists.ToString(),
                Artists[0].Name)).Content;
            LastArtists = (await Task.WhenAll(artistsTasks)).Select(x => x.Content).ToList();
            LastTrack = (await LastFMService.Instance.LastfmClient.Track.GetInfoAsync(Name, Artists.ToString()))
                .Content;
        }
        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
            return false;
        }

        return true;
    }

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

    public Task<bool> GetAvailable()
    {
        return Task.FromResult(true);
    }
}