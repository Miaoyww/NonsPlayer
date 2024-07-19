using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using WinUIEx.Messaging;

namespace NonsPlayer;

public sealed partial class MainWindow : WindowEx
{
    public MainWindow()
    {
        InitializeComponent();
        Content = null;
        Title = "AppDisplayName".GetLocalized();
        KeyHookService.Instance.Init();
        ExtendsContentIntoTitleBar = true;
    }
}