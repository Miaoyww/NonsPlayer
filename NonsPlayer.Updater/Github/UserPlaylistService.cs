using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Github;

public class GithubVersion : IJsonOnDeserialized
{
    public string Version { get; set; }

    public long PackageSize { get; set; }

    public string PackageHash { get; set; }

    [JsonIgnore] public string ReleasePageURL => $"https://github.com/Miaoyww/NonsPlayer/releases/tag/{Version}";

    public void OnDeserialized()
    {
    }
}