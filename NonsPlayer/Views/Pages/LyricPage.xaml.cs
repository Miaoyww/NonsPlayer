using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class LyricPage : Page
{
    public LyricPage()
    {
        ViewModel = App.GetService<LyricViewModel>();
        InitializeComponent();
    }

    public LyricViewModel ViewModel { get; }
}