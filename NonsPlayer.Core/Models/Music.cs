using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Exceptions;

namespace NonsPlayer.Core.Models;

public class Music : INonsModel
{
    public Album Album;
    public Artist[] Artists;
    public string FileType;
    public bool IsEmpty;
    public bool IsLiked;
    public Lyrics Lyrics;
    public MusicQualityLevel[] QualityLevels;
    public TimeSpan TotalTime;
    public string Url;

    public string TotalTimeString => TotalTime.ToString(@"m\:ss");
    public string CacheId => Id + "_music";
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    public string AlbumName => Album?.Name;

    public static Music CreateEmpty()
    {
        return new Music
        {
            Name = "暂无歌曲",
            Artists = new[] {Artist.CreatEmpty()},
            IsEmpty = true
        };
    }

    public async Task GetDetailsAsync()
    {
        if (Id == null) throw new MusicIdNullException("音乐Id为空, 请在调用此函数调用CreateAsync()");

        await Task.WhenAll(GetFileInfo(), GetLyric());
    }

    public async Task GetFileInfo()
    {
        var musicFile = (JObject) (await Apis.Music.Url(new[] {Id}, Nons.Instance))["data"][0];
        Url = musicFile["url"].ToString();
        FileType = musicFile["type"].ToString();
    }

    public async Task GetLyric()
    {
        // Lyrics = new Lyrics(await Apis.Lyric.GetLyric(Id.ToString(), Nons.Instance));
    }
}