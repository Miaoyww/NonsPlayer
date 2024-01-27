using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;

namespace NonsPlayer.Components.Views;

public sealed partial class UserPlaylistBar : UserControl
{
    public UserPlaylistBar()
    {
        ViewModel = App.GetService<UserPlaylistBarViewModel>();
        InitializeComponent();
    }

    public UserPlaylistBarViewModel ViewModel { get; }
}