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
    public List<Artist> Artists;
    public string FileType;
    public bool IsEmpty;
    public bool IsLiked;
    public Lyrics Lyrics;
    public MusicQualityLevel[] QualityLevels;
    public TimeSpan TotalTime;
    public string Url;

    private Music()
    {
    }

    public string TotalTimeString => TotalTime.ToString(@"m\:ss");
    public string CacheId => Id + "_music";
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    public string AlbumName => Album?.Name;

    public static async Task<Music> CreateAsync(JObject playlistMusicTrack)
    {
        var music = new Music();
        await music.ParseJObjectAsync(playlistMusicTrack);
        return music;
    }

    public static async Task<Music> CreateAsync(long id)
    {
        var music = new Music();
        JObject result;
        try
        {
            result = (JObject) (await Apis.Music.Detail(new[] {id}, Nons.Instance))["songs"][0];
        }
        catch (InvalidCastException)
        {
            throw new InvalidCastException($"未能发现此音乐{id}");
        }

        await music.ParseJObjectAsync(result);
        return music;
    }

    public static Music CreateEmpty()
    {
        var music = new Music();
        music.Name = "暂无歌曲";
        music.Artists = new List<Artist>
        {
            Artist.CreatEmpty()
        };
        music.IsEmpty = true;
        return music;
    }

    private async Task ParseJObjectAsync(JObject item)
    {
        Id = long.Parse(item["id"].ToString());
        Name = (string) item["name"];
        TotalTime = TimeSpan.FromMilliseconds(int.Parse(item["dt"].ToString()));

        Album = new Album
        {
            Name = (string) item["al"]["name"],
            Id = (int) item["al"]["id"],
            AvatarUrl = (string) item["al"]["picUrl"],
        };

        Artists = ((JArray) item["ar"]).Select(item =>  Artist.CreatAsync((JObject)item)).ToList();
        IsEmpty = false;
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