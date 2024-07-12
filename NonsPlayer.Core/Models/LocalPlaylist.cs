using System.Text.Json;
using System.Text.Json.Serialization;
using Newtonsoft.Json;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using JsonSerializer = System.Text.Json.JsonSerializer;

namespace NonsPlayer.Core.Models;

public class LocalPlaylist : IPlaylist
{
    // 本地歌单数据的路径
    public string Path;
    private List<IMusic> _musics;
    public DateTime CreateTime { get; set; }
    public string Creator { get; set; }
    public string Description { get; set; }
    public long[] MusicTrackIds { get; set; }
    public string[] Tags { get; set; }

    List<IMusic> IPlaylist.Musics
    {
        get => _musics;
        set => _musics = value;
    }

    [JsonPropertyName("musics")] public List<LocalMusic> Musics { get; set; }

    [JsonPropertyName("music_count")] public int MusicCount => Musics.Count;

    // 无缝播放模式
    [JsonPropertyName("mix_enable")] public bool MixEnable { get; set; }

    public LocalPlaylist(string path, string name)
    {
        Name = name;
        Path = path;
        Id = $"{Name}_{Path}";
        Musics = new List<LocalMusic>();
    }

    public static LocalPlaylist Load(string path)
    {
        return JsonSerializer.Deserialize<LocalPlaylist>(File.ReadAllText(path));
    }


    public void Save()
    {
        var options = new JsonSerializerOptions
        {
            WriteIndented = true,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingDefault
        };

        string jsonString = JsonSerializer.Serialize(this, options);
        File.WriteAllText($"{Path}\\{Name}.json", jsonString);
    }

    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
}