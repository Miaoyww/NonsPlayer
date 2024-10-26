using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using System.Text.Json.Serialization;
namespace NonsPlayer.Components.Models;

public class LocalFolderModel
{
    [JsonIgnore] public string Index;

    [JsonPropertyName("path")] public string Path { get; set; }

    public LocalFolderModel This;

    public LocalFolderModel(string path, string index = "")
    {
        Index = index;
        Path = System.IO.Path.GetFullPath(path);
        This = this;
    }
}