using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Windows.UI.Core;

namespace NonsPlayer.Views.Pages;

public sealed partial class LocalPage : Page
{
    public LocalPage()
    {
        ViewModel = App.GetService<LocalViewModel>();
        InitializeComponent();
        RecentlyPlayTextBlock.Text = "RecentlyPlay".GetLocalized();
        CountTextBlock.Text = "Count".GetLocalized();
        PathTextBlock.Text = "Path".GetLocalized();
        NameTextBlock.Text = "Name".GetLocalized();
    }

    public LocalViewModel ViewModel { get; }
}