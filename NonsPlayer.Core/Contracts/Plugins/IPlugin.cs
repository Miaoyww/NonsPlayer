namespace NonsPlayer.Core.Contracts.Plugins;

public interface IPlugin
{
    PluginMetadata GetMetadata();
    void Initialize();
    void Uninitialize();
}

public class PluginMetadata
{
    public PluginMetadata This { get; set; }

    public PluginMetadata()
    {
        This = this;
    }

    public string Name { get; set; }
    public string Author { get; set; }
    public string Description { get; set; }
    public Version Version { get; set; }
    public TimeSpan UpdateTime { get; set; }
    public Uri Repository { get; set; }
}