using System.Diagnostics;
using ABI.System.Collections.Generic;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework.Api;
using NonsPlayer.Helpers;

namespace NonsPlayer.Framework.Model;

public class PlayList
{
    /// <summary>
    ///     歌单创建时间
    /// </summary>
    public DateTime CreateTime;

    /// <summary>
    ///     歌单歌曲全部Id
    /// </summary>
    public long[] MusicTrackIds;

    /// <summary>
    ///     歌单标签
    /// </summary>
    public string[] Tags;

    /// <summary>
    ///     歌单名称
    /// </summary>
    public string Name
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单Id
    /// </summary>
    public long Id
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单封面Url
    /// </summary>
    public string PicUrl
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单描述
    /// </summary>
    public string Description
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单创建者
    /// </summary>
    public string Creator
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单歌曲数量
    /// </summary>
    public int MusicsCount => MusicTrackIds.Length;


    public Music[] Musics
    {
        get;
        set;
    }

    /// <summary>
    ///     通过Id打开歌单
    /// </summary>
    public async Task LoadAsync(long in_id)
    {
        Id = in_id;
        var (playlistDetail, apiRequestElapsed) =
            await Tools.MeasureExecutionTimeAsync(Apis.Playlist.Detail(Id, Nons.Instance));
        Debug.WriteLine($"歌单Api请求({Id})所用时间: {apiRequestElapsed.Milliseconds}ms");
        playlistDetail = (JObject)playlistDetail["playlist"];
        Name = playlistDetail["name"].ToString();
        Description = playlistDetail["description"].ToString();
        Tags = ((JArray)playlistDetail["tags"]).Select(tag => tag.ToString()).ToArray();
        PicUrl = playlistDetail["coverImgUrl"].ToString();
        CreateTime = DateTimeOffset.FromUnixTimeMilliseconds((long)playlistDetail["createTime"]).DateTime;
        Creator = playlistDetail["creator"]["nickname"].ToString();
        MusicTrackIds = ((JArray)playlistDetail["trackIds"]).Select(track => long.Parse(track["id"].ToString())).ToArray();
    }

    public async Task InitMusicsAsync()
    {
        var tracks = (JArray)(await Apis.Music.Detail(MusicTrackIds, Nons.Instance))["songs"];
        Musics = new Music[tracks.Count];
        var (results, elapsed) = await Tools.MeasureExecutionTimeAsync(Task.WhenAll(
            tracks.Select(track => Task.Run(() => new Music((JObject)track))).ToList()));
        Array.Copy(results, Musics, results.Length);
        Debug.WriteLine($"实例化歌单({Id})每首歌曲所用时间: {elapsed.Milliseconds}ms");
    }
    public Stream GetPic(int x = 0, int y = 0)
    {
        var IMGSIZE = "?param=300y300";
        if (x == 0)
        {
            return HttpRequest.StreamHttpGet(PicUrl + IMGSIZE);
        }

        return HttpRequest.StreamHttpGet(PicUrl + $"?param={x}y{y}");
    }
}