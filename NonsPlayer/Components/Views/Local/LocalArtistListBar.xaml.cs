using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using Windows.UI.Core;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LocalArtistListBar : UserControl
{
    public LocalArtistListBar()
    {
        ViewModel = App.GetService<LocalArtistListBarViewModel>();
        ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
        InitializeComponent();
        ArtistTextBlock.Text = "Artist".GetLocalized();
    }

    public ObservableCollection<LocalArtistModel> Models
    {
        set => ViewModel.Init(value);
    }


    public LocalArtistListBarViewModel ViewModel { get; }
}