using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.ViewModels;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

public sealed partial class BestArtistCard : UserControl
{
    public BestArtistCardViewModel ViewModel { get; }

    public BestArtistCard()
    {
        ViewModel = App.GetService<BestArtistCardViewModel>();
        InitializeComponent();
    }
    public ImageSource ArtistBrush =>
        new BitmapImage(new Uri("https://p2.music.126.net/wSi1HwucwrhlayiluXDKEQ==/109951166444443423.jpg"));
}