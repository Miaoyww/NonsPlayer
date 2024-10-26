using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

public sealed partial class BestMusicCard : UserControl
{
    public BestMusicCard()
    {
        ViewModel = App.GetService<BestMusicCardViewModel>();
        InitializeComponent();
    }

    public BestMusicCardViewModel ViewModel { get; }

    public IMusic Music
    {
        set
        {
            if (value != null)
            {
                ViewModel.CurrentMusic = value;
            }
        }
    }
}