using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class SearchPage : Page
{
    public SearchPage()
    {
        ViewModel = App.GetService<SearchViewModel>();
        InitializeComponent();
    }

    public SearchViewModel ViewModel { get; }
}