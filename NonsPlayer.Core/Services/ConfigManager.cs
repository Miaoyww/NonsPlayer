using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace NonsPlayer.Core.Services;

public class Config
{
    [JsonPropertyName("key")] public string Key { get; set; }

    [JsonPropertyName("value")] private string Value { get; set; }

    [JsonPropertyName("description")] public string Description { get; set; }


    public Config(string key, string value, string description = "")
    {
        Key = key;
        Value = value;
        Description = description;
    }

    public void Set(string value)
    {
        Value = value;
    }

    public string Get()
    {
        return Value;
    }
}

public class ConfigManager
{
    private readonly string _dataPath;


    private readonly string _configFilePath;

    private Dictionary<string, Config> _config;

    public static ConfigManager Instance { get; } = new();

    public ConfigManager()
    {
        _dataPath = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData) + "/NonsPlayer";
        _configFilePath = _dataPath + "/config.json";
    }

    public void LoadConfigAsync()
    {
        if (File.Exists(_configFilePath))
        {
            try
            {
                string json = File.ReadAllText(_configFilePath);
                _config = JsonSerializer.Deserialize<Dictionary<string, Config>>(json);
            }
            catch
            {
                _config = DefaultConfig();
            }
        }
        else
        {
            if (!File.Exists(_configFilePath))
            {
                Directory.CreateDirectory(Path.GetDirectoryName(_configFilePath));
                File.Create(_configFilePath).Close();
            }

            _config = DefaultConfig();
        }
    }

    public Config GetConfig(string key)
    {

        return _config[key];
    }

    public void SaveConfig()
    {
        if (!File.Exists(_configFilePath))
        {
            Directory.CreateDirectory(Path.GetDirectoryName(_configFilePath));
            File.Create(_configFilePath).Close();
        }

        var options = new JsonSerializerOptions
        {
            Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
        };
        string json = JsonSerializer.Serialize(_config, options);
        File.WriteAllText(_configFilePath, json, Encoding.UTF8);
    }

    public Dictionary<string, Config> DefaultConfig()
    {
        Dictionary<string, Config> config = new();
        List<Config> configs = new()
        {
            new("adapterPath", _dataPath + "/adapters", "适配器路径"),
            new("pluginPath", _dataPath + "/plugins", "插件路径"),
            new("data", _dataPath + "/data", "数据路径"),
            new("theme", "Light", "主题"),
            new("disabledAdapters", "", "禁用的适配器"),
            new("disabledPlugins", "", "禁用的插件"),
        };
        foreach (var item in configs)
        {
            config[item.Key] = item;
        }

        return config;
    }
}