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
    }

    public LocalViewModel ViewModel { get; }
}