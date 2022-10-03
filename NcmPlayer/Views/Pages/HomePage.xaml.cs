using Microsoft.UI.Xaml.Controls;
using NcmPlayer.ViewModels;

namespace NcmPlayer.Views;

public sealed partial class HomePage : Page
{
    public HomeViewModel ViewModel
    {
        get;
    }

    public VariableSizedWrapGrid Panel_MusicList;
    public HomePage()
    {
        ViewModel = App.GetService<HomeViewModel>();
        InitializeComponent();
        Panel_MusicList = panel_nicePlaylists;
    }
}
