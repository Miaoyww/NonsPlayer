using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Github;

public class ReleaseFile : IJsonOnDeserialized
{
    public ReleaseVersion Release;
    public string From { get; set; }

    public string To { get; set; }

    public string Path { get; set; }
    public bool IsMoving { get; set; }

    public void OnDeserialized()
    {
    }
}