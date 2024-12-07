using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.DataBase.Models;
using NonsPlayer.Services;
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

    public DbFolderModel ConvertToDbFolderModel()
    {
        return new DbFolderModel
        {
            Path = this.Path,
            LastModified = (int)new FileInfo(this.Path).LastWriteTimeUtc.Subtract(new DateTime(1970, 1, 1)).TotalSeconds
        };
    }
}