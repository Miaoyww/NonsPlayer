using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer;

public sealed partial class MainWindow : WindowEx
{
    public UiHelper UiHelper = UiHelper.Instance;

    public MainWindow()
    {
        InitializeComponent();
        Content = null;
        Title = "AppDisplayName".GetLocalized();
        KeyHookService.Instance.Init();
        ExtendsContentIntoTitleBar = true;

    }

    private void MainWindow_OnSizeChanged(object sender, WindowSizeChangedEventArgs args)
    {
        
    }

}