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

    public ImageSource ArtistBrush =>
        new BitmapImage(new Uri("https://p2.music.126.net/wSi1HwucwrhlayiluXDKEQ==/109951166444443423.jpg"));

    public ImageBrush BestResultBrush =>
        new()
        {
            ImageSource =
                new BitmapImage(new Uri("https://p3.music.126.net/BdwgC6JBYeBXqzkHRNwDZg==/109951168769184685.jpg"))
        };
}