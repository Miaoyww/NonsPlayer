using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class HomePage : Page
{
    public HomePage()
    {
        ViewModel = App.GetService<HomeViewModel>();
        InitializeComponent();
    }

    public HomeViewModel ViewModel { get; }

    private void CardHide(object sender, PointerRoutedEventArgs e)
    {
        AnimationsHelper.CardHide(sender, e);
    }

    private void CardShow(object sender, PointerRoutedEventArgs e)
    {
        AnimationsHelper.CardShow(sender, e);
    }
}