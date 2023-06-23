using System.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Framework.Model;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

public sealed partial class MusicItemCard : Page
{
    public MusicItemCardViewModel ViewModel
    {
        get;
    }

    public MusicItemCard(Music one)
    {
        ViewModel = App.GetService<MusicItemCardViewModel>();
        InitializeComponent();
        ViewModel.Init(one);
    }
    // private void CardShow(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardShow(sender, e);
    // private void CardHide(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardHide(sender, e);

}
