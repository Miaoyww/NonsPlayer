using System.IO;
using System.Text;
using System.Text.Json;
using NonsPlayer.Core.Resources;

namespace NonsPlayer.Core.Services;

public class ConfigManager
{
    public static ConfigManager Instance { get; } = new();
    public LocalSettings Settings;

    public ConfigManager()
    {
        Settings = new();
    }
    public void Load()
    {
        if (File.Exists(Settings.ConfigFilePath))
        {
            try
            {
                var json = File.ReadAllText(Settings.ConfigFilePath);
                Settings = JsonSerializer.Deserialize<LocalSettings>(json);
            }
            catch
            {
                // ignore
            }
        }
        else
        {
            if (!File.Exists(Settings.ConfigFilePath))
            {
                Directory.CreateDirectory(Path.GetDirectoryName(Settings.ConfigFilePath));
                File.Create(Settings.ConfigFilePath).Close();
            }
            // ignore if not exists
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