using Microsoft.UI.Xaml.Controls;

using NcmPlayer.ViewModels;

namespace NcmPlayer.Views;

public sealed partial class MusicListDetailPage : Page
{
    public MusicListDetailViewModel ViewModel
    {
        get;
    }

    public MusicListDetailPage()
    {
        ViewModel = App.GetService<MusicListDetailViewModel>();
        InitializeComponent();
    }
}
