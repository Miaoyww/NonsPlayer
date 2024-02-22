using System.Text.Json.Serialization;
using NonsPlayer.Updater.Metadata;

namespace NonsPlayer.Updater.Github;

public class ReleaseFile : IJsonOnDeserialized
{
    public ReleaseVersion Release;
    public LocalFile File;

    public void OnDeserialized()
    {
    }
}