using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Models;

public class Music : INonsModel
{
    public Album Album;
    public Artist[] Artists;
    public string FileType;
    public bool IsEmpty;
    public bool IsLiked;
    public MusicQualityLevel[] QualityLevels;
    public TimeSpan TotalTime;
    public string Trans;
    public string Url;
    public Lyric? Lyric;
    public string TotalTimeString => TotalTime.ToString(@"m\:ss");
    public string CacheId => Id + "_music";
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    public string AlbumName => Album?.Name;

    public static Music CreateEmpty()
    {
        return new Music
        {
            Name = "暂无歌曲",
            Artists = new[] { Artist.CreatEmpty() },
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
        var musicFile = (JObject)(await Apis.Music.Url(Id, MusicQualityLevel.ExHigh, NonsCore.Instance))["data"][0];
        Url = musicFile["url"].ToString();
        FileType = musicFile["type"].ToString();
    }

    public async Task GetLyric()
    {
        if (Lyric != null)
        {
            return;
        }

        var response = await Apis.Lyric.GetLyric(Id.ToString(), NonsCore.Instance);
        var originalLyric = response.ContainsKey("lrc")
            ? AMLL.Parsers.Lrc.ParseLrc(((JObject)response.GetValue("lrc")).GetValue("lyric").ToString(), TotalTime)
            : null;
        var transLyric = response.ContainsKey("tlyric")
            ? AMLL.Parsers.Lrc.ParseLrc(((JObject)response.GetValue("tlyric")).GetValue("lyric").ToString(), TotalTime)
            : null;
        var yrc = response.ContainsKey("yrc")
            ? AMLL.Parsers.Yrc.ParseYrc(((JObject)response.GetValue("yrc")).GetValue("lyric").ToString())
            : null;
        if (yrc != null)
        {
            yrc.CombinePure(originalLyric);
            yrc.AddTrans(transLyric);
            Lyric = yrc;

            return;
        }

        originalLyric.AddTrans(transLyric);

        Lyric = originalLyric;
    }
}