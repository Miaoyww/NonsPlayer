using NcmPlayer.Helpers;

namespace NcmPlayer;

public sealed partial class MainWindow : WindowEx
{
    public MainWindow()
    {
        InitializeComponent();
        Content = null;
        Title = "AppDisplayName".GetLocalized();
    }

}