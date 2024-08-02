using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class LocalFolderModel
{
    public string Index;

    public string Name;

    public string Path;

    public LocalFolderModel This;

    public LocalFolderModel(string index, string name, string path)
    {
        Index = index;
        Name = name;
        Path = path;
        This = this;
    }
}