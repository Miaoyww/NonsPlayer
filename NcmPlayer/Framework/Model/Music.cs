using NcmApi;
using NcmPlayer.Framework.Enums;
using NcmPlayer.Helpers;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;

namespace NcmPlayer.Framework.Model;

public class Music
{
    /// <summary>
    ///     歌曲专辑
    /// </summary>
    public Album Album;

    /// <summary>
    ///     歌曲作者
    /// </summary>
    public List<Artist> Artists;

    /// <summary>
    ///     歌曲封面Url
    /// </summary>
    public string CoverUrl;

    /// <summary>
    ///     歌曲总时长
    /// </summary>
    public TimeSpan DuartionTime;

    /// <summary>
    ///     歌曲总时长的String版 mm:ss
    /// </summary>
    public string DuartionTimeString;

    /// <summary>
    ///     下载文件的扩展名
    /// </summary>
    public string FileType;

    /// <summary>
    ///     歌曲Id
    /// </summary>
    public long Id;

    /// <summary>
    ///     是否收藏歌曲
    /// </summary>
    public bool IsLiked;

    /// <summary>
    ///     歌曲名称
    /// </summary>
    public string Name;

    /// <summary>
    ///     歌曲Url
    /// </summary>
    public string Url;

    public Music(JObject playlistMusicTrack, bool recommend = false)
    {
        Name = (string)playlistMusicTrack["name"];
        Id = (int)playlistMusicTrack["id"];
        CoverUrl = (string)playlistMusicTrack["al"]["picUrl"];
        DuartionTime = TimeSpan.FromMilliseconds(int.Parse(playlistMusicTrack["dt"].ToString()));
        DuartionTimeString = DuartionTime.ToString(@"m\:ss");

        Album = new Album();
        Album.Name = (string)playlistMusicTrack["al"]["name"];
        Album.Id = (int)playlistMusicTrack["al"]["id"];
        Album.PicUrl = (string)playlistMusicTrack["al"]["picUrl"];

        Artists = new List<Artist>();
        var artists = (JArray)playlistMusicTrack["ar"];
        foreach (JObject artist in artists)
        {
            Artist one = new();
            one.Name = (string)artist["name"];
            one.Id = (int)artist["id"];
            Artists.Add(one);
        }

        Task.Run(() =>
        {
            GetFileInfo();
        });
    }

    public Music()
    {
    }

    /// <summary>
    ///     歌曲作者名称
    /// </summary>
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));

    /// <summary>
    ///     歌曲专辑
    /// </summary>
    public string AlbumName => Album?.Name;

    /// <summary>
    ///     支持的歌曲质量
    /// </summary>
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
            musicDetail = (JObject)((JArray)Api.Music.Detail(new[] {Id}, ResEntry.ncm).Result["songs"])[0];
        }
        catch (InvalidCastException)
        {
            throw new InvalidCastException($"未能发现此音乐{in_id}");
        }

        Name = (string)musicDetail["name"];
        Id = (int)musicDetail["id"];
        CoverUrl = (string)musicDetail["al"]["picUrl"];
        DuartionTime = TimeSpan.FromMilliseconds(int.Parse(musicDetail["dt"].ToString()));
        DuartionTimeString = DuartionTime.ToString(@"m\:ss");

        Album = new Album();
        Album.Name = (string)musicDetail["al"]["name"];
        Album.Id = (int)musicDetail["al"]["id"];
        Album.PicUrl = (string)musicDetail["al"]["picUrl"];

        Artists = new List<Artist>();
        var artists = (JArray)musicDetail["ar"];
        foreach (JObject artist in artists)
        {
            Artist one = new();
            one.Name = (string)artist["name"];
            one.Id = (int)artist["id"];
            Artists.Add(one);
        }

        await GetFileInfo();
    }

    public async Task GetFileInfo()
    {
        var musicFile = (JObject)(await Api.Music.Url(new[] {Id}, ResEntry.ncm))["data"][0];
        Url = musicFile["url"].ToString();
        FileType = musicFile["type"].ToString();
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

    /*
    public string GetMp3()
    {
        string _musicUrl = MusicUrl;
        string path = AppConfig.MusicsPath(Id, MusicType);
        if (!File.Exists(path))
        {
            if (!Directory.Exists(AppConfig.MusicsDirectory))
            {
                Directory.CreateDirectory(AppConfig.MusicsDirectory);
            }
            FileStream fs = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
            Stream musicStream = HttpRequest.StreamHttpGet(_musicUrl).Result;
            byte[] bArr = new byte[1024];
            int size = musicStream.Read(bArr, 0, bArr.Length);
            while (size > 0)
            {
                //stream.Write(bArr, 0, size);
                fs.Write(bArr, 0, size);
                size = musicStream.Read(bArr, 0, bArr.Length);
            }
            fs.Close();
            musicStream.Close();
        }
        return path;
    }*/

    /*
    public Lrcs GetLrc
    {
        get
        {
            if (lrc == null)
            {
                object? content = Api.Lyric.Lrc(Id, ResEntry.ncm).Property("lrc");
                if (content != null)
                {
                    lrcString = ((JProperty)content).Value["lyric"].ToString();
                    lrc = new(lrcString);
                }
                else
                {
                    lrcString = "[99:99.000] 暂无歌词";
                    lrc = new(lrcString);
                }
            }
            return lrc;
        }
    }*/
}