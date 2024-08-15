using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using System.Text.Json.Serialization;

namespace NonsPlayer.Components.Models;

public class LocalFolderModel
{
    [JsonIgnore] public string Index;

    [JsonPropertyName("name")] public string Name { get; set; }

    [JsonPropertyName("path")] public string Path { get; set; }

    public LocalFolderModel This;

    public LocalFolderModel(string name, string path, string index = "")
    {
        Index = index;
        Name = name;
        Path = path;
        This = this;
    }
}