using System.IO;
using System.Text;
using System.Text.Json;
using NonsPlayer.Core.Contracts.Managers;
using NonsPlayer.Core.Resources;

namespace NonsPlayer.Core.Services;

public class ConfigManager: IConfigManager
{
    #region 事件注册

    public delegate void ConfigManagerEventHandler(string param);

    public event ConfigManagerEventHandler ConfigLoadFailed;
    #endregion
    public static ConfigManager Instance { get; } = new();
    public LocalSettings Settings;

    public ConfigManager()
    {
        Settings = new();
    }

    public void Load()
    {
        try
        {
            if (File.Exists(Settings.ConfigFilePath))
            {
                var json = File.ReadAllText(Settings.ConfigFilePath);
                Settings = JsonSerializer.Deserialize<LocalSettings>(json);
            }
            else
            {
                Directory.CreateDirectory(Path.GetDirectoryName(Settings.ConfigFilePath));
                File.Create(Settings.ConfigFilePath).Close();
            }
        }
        catch(Exception e)
        {
            ConfigLoadFailed?.Invoke(e.ToString());
        }

    }

    public void Save()
    {
        if (!File.Exists(Settings.ConfigFilePath))
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Settings.ConfigFilePath));
            File.Create(Settings.ConfigFilePath).Close();
        }

        var options = new JsonSerializerOptions
        {
            Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
        };

        var json = JsonSerializer.Serialize(Settings, options);
        File.WriteAllText(Settings.ConfigFilePath, json, Encoding.UTF8);
    }
}