using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Activation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;
using NonsPlayer.Views.Pages;
using System.Text.Json.Serialization;
using WinRT;
using UnhandledExceptionEventArgs = Microsoft.UI.Xaml.UnhandledExceptionEventArgs;
using NonsPlayer.Core.Resources;
using System.Text.Json;
using System.Text;
using NonsPlayer.Core.Contracts.Managers;

namespace NonsPlayer;

public class AppConfig : IConfigManager
{
    public static AppConfig Instance { get; } = new();
    public AppSettings AppSettings;

    public AppConfig()
    {
        AppSettings = new();
        Initialize();
    }

    public string? IgnoreVersion { get; set; }
    public string? AppVersion { get; set; }
    public string ConfigPath = Path.Combine(ConfigManager.Instance.Settings.MainPath, "app_config.json");

    private void Initialize()
    {
        AppVersion = typeof(AppConfig).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion;
        Load();
    }

    public void Load()
    {
        try
        {
            if (File.Exists(ConfigPath))
            {
                var json = File.ReadAllText(ConfigPath);
                AppSettings = JsonSerializer.Deserialize<AppSettings>(json);
            }
            else
            {
                Directory.CreateDirectory(Path.GetDirectoryName(ConfigPath));
                File.Create(ConfigPath).Close();
            }
        }
        catch (Exception e)
        {
            // ignore
        }
    }

    public void Save()
    {
        if (!File.Exists(ConfigPath))
        {
            Directory.CreateDirectory(Path.GetDirectoryName(ConfigPath));
            File.Create(ConfigPath).Close();
        }

        var options = new JsonSerializerOptions
        {
            Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
        };

        var json = JsonSerializer.Serialize(AppSettings, options);
        File.WriteAllText(ConfigPath, json, Encoding.UTF8);
    }
}