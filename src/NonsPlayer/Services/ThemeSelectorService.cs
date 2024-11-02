using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Services;

public class ThemeSelectorService : IThemeSelectorService
{
    // private const string SettingsKey = "AppBackgroundRequestedTheme";

    public ElementTheme Theme { get; set; } = ElementTheme.Default;

    public void Initialize()
    {
        Theme = LoadThemeFromSettings();
    }

    public void SetTheme(ElementTheme theme)
    {
        Theme = theme;

        SetRequestedTheme();
        SaveThemeInSettings(Theme);
    }

    public void SetRequestedTheme()
    {
        if (App.MainWindow.Content is FrameworkElement rootElement)
        {
            rootElement.RequestedTheme = Theme;
            TitleBarHelper.UpdateTitleBar(Theme);
        }
    }

    private ElementTheme LoadThemeFromSettings()
    {
        var themeName = ConfigManager.Instance.Settings.Theme;
        
        if (Enum.TryParse(themeName, out ElementTheme cacheTheme)) return cacheTheme;
        
        return ElementTheme.Default;
    }

    private void SaveThemeInSettings(ElementTheme theme)
    {
        ConfigManager.Instance.Settings.Theme = Theme.ToString();
        ConfigManager.Instance.Save();
    }
}