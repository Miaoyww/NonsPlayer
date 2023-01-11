using Microsoft.UI.Xaml.Controls;
using NcmPlayer.ViewModels;

namespace NcmPlayer.Views.Pages;

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