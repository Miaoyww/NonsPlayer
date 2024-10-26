using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Github;

public class ReleaseVersion : IJsonOnDeserialized
{
    public string Version { get; set; }


    public string Architecture { get; set; }


    public DateTimeOffset BuildTime { get; set; }

    public string Install { get; set; }


    public long InstallSize { get; set; }


    public string InstallHash { get; set; }


    public string Portable { get; set; }


    public long PortableSize { get; set; }


    public string PortableHash { get; set; }


    [JsonIgnore] public string ReleasePageURL => $"https://github.com/Miaoyww/NonsPlayer/releases/tag/{Version}";

    public void OnDeserialized()
    {
    }
}