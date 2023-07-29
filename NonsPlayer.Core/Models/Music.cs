using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class Music
{
    [JsonPropertyName("album")]
    public Album Album;
    
    [JsonPropertyName("artists")]
    public List<Artist> Artists;
    
    [JsonPropertyName("cover")]
    public string CoverUrl;
    
    [JsonPropertyName("total_time")]
    public TimeSpan TotalTime;
    
    public string TotalTimeString => TotalTime.ToString(@"m\:ss");
    
    public string FileType;
    
    [JsonPropertyName("id")]
    public long Id;
    
    public bool IsLiked;
    
    [JsonPropertyName("name")]
    public string Name;
    
    [JsonPropertyName("url")]
    public string Url;
    
    [JsonPropertyName("is_empty")]
    public bool IsEmpty;
    
    [JsonPropertyName("lyrics")]
    public Lyrics Lyrics;
    
    public Music(JObject playlistMusicTrack)
    {
        Name = (string)playlistMusicTrack["name"];
        Id = (int)playlistMusicTrack["id"];
        CoverUrl = (string)playlistMusicTrack["al"]["picUrl"];
        TotalTime = TimeSpan.FromMilliseconds(int.Parse(playlistMusicTrack["dt"].ToString()));

        Album = new Album
        {
            Name = (string)playlistMusicTrack["al"]["name"],
            Id = (int)playlistMusicTrack["al"]["id"],
            PicUrl = (string)playlistMusicTrack["al"]["picUrl"]
        };

        Artists = ((JArray)playlistMusicTrack["ar"]).Select(t => new Artist
        {
            Name = (string)t["name"],
            Id = (int)t["id"]
        }).ToList();
        IsEmpty = false;

    }

    /// <summary>
    /// 定义一个空Music
    /// </summary>
    public Music()
    {
        Name = "暂未播放";
        Artists = new()
        {
            new Artist {Name = "无"}
        };
        IsEmpty = true;
    }
    
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));


    public string AlbumName => Album?.Name;
    
    public MusicQualityLevel[] QualityLevels
    {
        get;
        set;
    }

    public async Task LoadIdAsync(long in_id)
    {
        Id = in_id;
        JObject musicDetail;
        try
        {
            musicDetail = (JObject)((JArray)Apis.Music.Detail(new[] { Id }, Nons.Instance).Result["songs"])[0];
        }
        catch (InvalidCastException)
        {
            throw new InvalidCastException($"未能发现此音乐{in_id}");
        }

        Name = (string)musicDetail["name"];
        Id = (int)musicDetail["id"];
        CoverUrl = (string)musicDetail["al"]["picUrl"];
        TotalTime = TimeSpan.FromMilliseconds(int.Parse(musicDetail["dt"].ToString()));
        Album = new Album
        {
            Name = (string)musicDetail["al"]["name"],
            Id = (int)musicDetail["al"]["id"],
            PicUrl = (string)musicDetail["al"]["picUrl"]
        };

        Artists = new List<Artist>();
        var artists = (JArray)musicDetail["ar"];
        foreach (var jToken in artists)
        {
            var artist = (JObject)jToken;
            Artist one = new()
            {
                Name = (string)artist["name"],
                Id = (int)artist["id"]
            };
            Artists.Add(one);
        }

        await GetFileInfo();
    }

    public async Task GetFileInfo()
    {
        var musicFile = (JObject)(await Apis.Music.Url(new[] { Id }, Nons.Instance))["data"][0];
        Url = musicFile["url"].ToString();
        FileType = musicFile["type"].ToString();
    }

    public async Task GetLyric()
    {
        var a = await Apis.Lyric.GetLyric(Id.ToString(), Nons.Instance);
        Lyrics = new Lyrics(a);
    }

    public Stream GetCover(int x = 0, int y = 0)
    {
        var IMGSIZE = "?param=300y300";
        if (x == 0)
        {
            return HttpRequest.StreamHttpGet(CoverUrl + IMGSIZE);
        }

        return HttpRequest.StreamHttpGet(CoverUrl + $"?param={x}y{y}");
    }
}