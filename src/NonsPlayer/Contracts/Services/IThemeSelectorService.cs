using Microsoft.UI.Xaml;

namespace NonsPlayer.Contracts.Services;

public interface IThemeSelectorService
{
    ElementTheme Theme { get; }

    void Initialize();

    void SetTheme(ElementTheme theme);

    void SetRequestedTheme();
}