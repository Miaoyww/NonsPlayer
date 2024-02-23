using System.Text.Json.Serialization;

namespace NonsPlayer.Updater.Metadata;

public class LocalFile
{
    public string To { get; set; }

    public string Path { get; set; }

    public string Hash { get; set; }

    public bool IsMoving { get; set; }
}