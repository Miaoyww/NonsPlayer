using System.Diagnostics;
using System.Text.Json.Serialization;


namespace NonsPlayer.Core.Contracts.Models.Music;

public interface IPlaylist : IMusicModel
{
    [JsonPropertyName("create_time")] public DateTime CreateTime { get; set; }
    [JsonPropertyName("creator")] public string Creator { get; set; }
    [JsonPropertyName("description")] public string Description { get; set; }
    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds { get; set; }
    [JsonPropertyName("tags")] public string[] Tags { get; set; }
    public List<IMusic> Musics { get; set; }
    public int MusicsCount => MusicTrackIds.Length;
    public string CacheId => Id + "_playlist";
    
    void InitializeMusics();
}