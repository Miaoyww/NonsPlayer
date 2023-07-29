using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class Playlist
{
    [JsonPropertyName("create_time")]
    public DateTime CreateTime;

    [JsonPropertyName("music_track_ids")]
    public long[] MusicTrackIds;

    [JsonPropertyName("tags")]
    public string[] Tags;

    [JsonPropertyName("name")]
    public string Name
    {
        get;
        set;
    }

    [JsonPropertyName("id")]
    public long Id
    {
        get;
        set;
    }

    [JsonPropertyName("pic_url")]
    public string PicUrl
    {
        get;
        set;
    }

    [JsonPropertyName("description")]
    public string Description
    {
        get;
        set;
    }

    [JsonPropertyName("creator")]
    public string Creator
    {
        get;
        set;
    }

    [JsonPropertyName("musics_count")]
    public int MusicsCount => MusicTrackIds.Length;

    [JsonPropertyName("musics")]
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