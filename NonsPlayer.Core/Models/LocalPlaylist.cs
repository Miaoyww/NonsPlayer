using System.Text.Json;
using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class LocalPlaylist : IPlaylist
{
    // 本地歌单数据的路径
    public string Path;
    [JsonPropertyName("musics")] public List<LocalMusic> Musics;
    public int MusicsCount => Musics.Count;

    public LocalPlaylist(string path, string name)
    {
        Name = name;
        Path = path;
        Musics = new List<LocalMusic>();
    }

    public static LocalPlaylist Load(string path)
    {
        return JsonSerializer.Deserialize<LocalPlaylist>(File.ReadAllText(path));
    }


    public void Save()
    {
        File.WriteAllText($"{Path}\\{Name}.json", JsonSerializer.Serialize(this));
    }
}