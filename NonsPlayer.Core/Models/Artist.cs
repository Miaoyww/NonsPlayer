using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class Artist : INonsModel
{
    [JsonPropertyName("desc")] public string Description { get; set; }
    [JsonPropertyName("musics")] public List<Music> HotMusics { get; set; }
    [JsonPropertyName("music_count")] public int MusicCount { get; set; }
    [JsonPropertyName("trans")] public string Trans { get; set; }

    public static Artist CreatEmpty()
    {
        return new Artist
        {
            Name = "未知艺术家"
        };
    }
}