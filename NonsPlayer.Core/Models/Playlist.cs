using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Core.Models;

public class Playlist : INonsModel
{
    [JsonPropertyName("create_time")] public DateTime CreateTime;
    [JsonPropertyName("creator")] public string Creator;
    [JsonPropertyName("description")] public string Description;
    public string Platform;
    public IAdapter Adapter;
    public bool IsCardMode;
    public List<Music> Musics;
    public JObject? PlaylistPure;

    public int MusicsCount => MusicTrackIds.Length;

    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds;
    [JsonPropertyName("tags")] public string[] Tags;
    public string CacheId => Id + "_playlist";
    public Playlist This => this;

    public Playlist(string platform)
    {
        Platform = platform;
        Adapter = AdapterService.Instance.GetAdapter(Platform);
    }

    /// <summary>
    /// 只会获取前1000歌曲信息
    /// </summary>
    public void InitTracks()
    {
        Musics = AdapterService.Instance.GetAdapter(Platform).Playlist.InitTracks(PlaylistPure);
    }
    
    public async Task InitTrackByIndexAsync(int start, int end)
    {
        var ids = MusicTrackIds.Skip(start).Take(end - start).ToArray();
        var musicTrackIdsGroup = ids.Select((id, index) => new { id, index })
            .GroupBy(x => x.index / 200)
            .Select(x => x.Select(v => v.id).ToArray())
            .ToArray();
        var musicJObjectTasks = musicTrackIdsGroup.Select(x => Apis.Music.Detail(x, NonsCore.Instance));
        var (result, elapsed) = await Task.WhenAll(musicJObjectTasks).MeasureExecutionTimeAsync();
        Musics.AddRange(result
            .Select(item => Adapter.Music.GetMusicList(item["songs"] as JArray))
            .SelectMany(x => x));
        Debug.WriteLine($"获取歌曲({start}-{end})信息获取完毕 {elapsed.TotalMilliseconds}ms");
    }
}