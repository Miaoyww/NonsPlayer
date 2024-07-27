using System.Text.Json;
using System.Text.Json.Serialization;
using Newtonsoft.Json;
using NonsPlayer.Core.Contracts.Adapters;
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
    public string[] MusicTrackIds { get; set; }
    public string[] Tags { get; set; }
    public IAdapter Adapter { get; set; }

    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
    public bool IsInitialized { get; set; }

    List<IMusic> IPlaylist.Musics
    {
        get => _musics;
        set => _musics = value;
    }



    public Task InitializePlaylist()
    {
        // ignore
        return null;
    }

    public Task InitializeMusics()
    {
        // ignore
        return null;
    }

    public Task InitializeTracks()
    {
        // ignore
        return null;
    }

    [JsonPropertyName("songs")] public List<LocalMusic> Songs { get; set; }

    [JsonPropertyName("song_count")] public int SongCount => Songs.Count;

    // 无缝播放模式
    [JsonPropertyName("mix_enable")] public bool MixEnable { get; set; }

    public LocalPlaylist(string path, string name)
    {
        Name = name;
        Path = path;
        Id = $"{Name}_{Path}";
        Songs = new List<LocalMusic>();
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