using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;

namespace NonsPlayer.Components.Views;

public sealed partial class PlayerBar : UserControl
{
    public PlayerBarViewModel ViewModel
    {
        get;
    }

    public PlayerBar()
    {
        ViewModel = App.GetService<PlayerBarViewModel>();
        InitializeComponent();
    }
}