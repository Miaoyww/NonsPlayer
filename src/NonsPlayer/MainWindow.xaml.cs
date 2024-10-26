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
}