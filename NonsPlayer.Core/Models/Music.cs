using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class Music
{
    [JsonPropertyName("album")] public Album Album;

    [JsonPropertyName("artists")] public List<Artist> Artists;

    [JsonPropertyName("total_time")] public TimeSpan TotalTime;

    public string TotalTimeString => TotalTime.ToString(@"m\:ss");

    public string FileType;

    [JsonPropertyName("id")] public long? Id;
    public string CacheId => Id.ToString() + "_music";
    public bool IsLiked;

    [JsonPropertyName("name")] public string Name;

    [JsonPropertyName("url")] public string Url;

    [JsonPropertyName("is_empty")] public bool IsEmpty;

    [JsonPropertyName("lyrics")] public Lyrics Lyrics;
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    public string AlbumName => Album?.Name;

    public MusicQualityLevel[] QualityLevels;

    private Music()
    {
    }

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
            result = (JObject)(await Apis.Music.Detail(new long[] {id}, Nons.Instance))["songs"][0];
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
            new()
            {
                Name = "未知艺术家"
            }
        };
        music.IsEmpty = true;
        return music;
    }

    private async Task ParseJObjectAsync(JObject item)
    {
        Id = long.Parse(item["id"].ToString());
        Name = (string)item["name"];
        TotalTime = TimeSpan.FromMilliseconds(int.Parse(item["dt"].ToString()));

        Album = new Album
        {
            Name = (string)item["al"]["name"],
            Id = (int)item["al"]["id"],
            CoverUrl = (string)item["al"]["picUrl"],
            SmallCoverUrl = (string)item["al"]["picUrl"] + "?param=40y40",
        };

        Artists = ((JArray)item["ar"]).Select(t => new Artist
        {
            Name = (string)t["name"],
            Id = (int)t["id"]
        }).ToList();
        IsEmpty = false;
    }

    public async Task GetDetailsAsync()
    {
        if (Id == null)
        {
            throw new MusicIdNullException("音乐Id为空, 请在调用此函数调用CreateAsync()");
        }

        await Task.WhenAll(GetFileInfo(), GetLyric());
    }

    public async Task GetFileInfo()
    {
        var musicFile = (JObject)(await Apis.Music.Url(new long?[] {Id}, Nons.Instance))["data"][0];
        Url = musicFile["url"].ToString();
        FileType = musicFile["type"].ToString();
    }

    public async Task GetLyric()
    {
        Lyrics = new Lyrics(await Apis.Lyric.GetLyric(Id.ToString(), Nons.Instance));
    }
}