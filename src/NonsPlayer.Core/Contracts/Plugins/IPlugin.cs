namespace NonsPlayer.Core.Contracts.Plugins;

public interface IPlugin
{
    void Initialize();
    void Uninitialize();
    
    PluginMetadata GetMetadata();
}

public class PluginMetadata
{
    public PluginMetadata This { get; set; }
    public PluginMetadata()
    {
        This = this;
    }
    
    /// <summary>
    /// 
    /// </summary>
    public string Name { get; set; }
    /// <summary>
    /// 用于显示在界面上的名字
    /// </summary>
    public string DisPlayName { get; set; }
    public string Author { get; set; }
    public string Description { get; set; }
    public Version Version { get; set; }
    public TimeSpan UpdateTime { get; set; }
    public Uri Repository { get; set; }
}