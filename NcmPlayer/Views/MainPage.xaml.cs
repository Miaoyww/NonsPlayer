using Microsoft.UI.Xaml.Controls;

using NcmPlayer.ViewModels;

namespace NcmPlayer.Views;

public sealed partial class MainPage : Page
{
    public MainViewModel ViewModel
    {
        get;
    }

    public MainPage()
    {
        ViewModel = App.GetService<MainViewModel>();
        InitializeComponent();
    }
}
