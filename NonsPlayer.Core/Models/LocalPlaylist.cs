using System.Text.Json;
using System.Text.Json.Serialization;
using Newtonsoft.Json;
using NonsPlayer.Core.Contracts.Models;
using JsonSerializer = System.Text.Json.JsonSerializer;

namespace NonsPlayer.Core.Models;

public class LocalPlaylist : INonsModel
{
    // 本地歌单数据的路径
    public string Path;
    [JsonPropertyName("musics")] public List<LocalMusic> Musics { get; set; }
    [JsonPropertyName("music_count")] public int MusicCount => Musics.Count;

    public LocalPlaylist(string path, string name)
    {
        Name = name;
        Path = path;
        Id = $"{Name}_{Path}".GetHashCode();
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
}