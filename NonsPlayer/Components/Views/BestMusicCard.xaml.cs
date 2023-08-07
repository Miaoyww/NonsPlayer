using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.ViewModels;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

public sealed partial class BestMusicCard : UserControl
{
    public BestMusicCardViewModel ViewModel { get; }

    public BestMusicCard()
    {
        ViewModel = App.GetService<BestMusicCardViewModel>();
        InitializeComponent();
    }
}