using Microsoft.UI.Xaml.Controls;
using NonsPlayer;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class MusicListDetailPage : Page
{
    public MusicListDetailViewModel ViewModel
    {
        get;
    }

    public StackPanel MusicsViewPanel;
    public MusicListDetailPage()
    {
        ViewModel = App.GetService<MusicListDetailViewModel>();
        InitializeComponent();
        MusicsViewPanel = MusicsPanel;
    }

}