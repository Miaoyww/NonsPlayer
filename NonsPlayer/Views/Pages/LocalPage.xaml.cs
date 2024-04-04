using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

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