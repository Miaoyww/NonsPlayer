namespace NonsPlayer.Core.Contracts.Adapters;

public class AdapterMetadata
{
    public string Name { get; set; }
    public string Author { get; set; }
    public string Description { get; set; }
    public Version Version { get; set; }
    public TimeSpan UpdateTime { get; set; }
    public Uri Repository { get; set; }

    public AdapterType Type { get; set; }
}

public enum AdapterType
{
    Common,
    OnlyMusic
}

public interface IAdapter
{
    AdapterMetadata GetMetadata();
}