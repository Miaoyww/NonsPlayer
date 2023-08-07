using NonsPlayer.Helpers;

namespace NonsPlayer;

public sealed partial class MainWindow : WindowEx
{
    public MainWindow()
    {
        InitializeComponent();
        Content = null;
        Title = "AppDisplayName".GetLocalized();
    }
}