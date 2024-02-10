using System.Collections.ObjectModel;
using Windows.ApplicationModel.DataTransfer;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

public sealed partial class MusicListBar : UserControl
{
    public MusicListBar()
    {
        ViewModel = App.GetService<MusicListBarViewModel>();
        InitializeComponent();
    }

    public ObservableCollection<MusicItem> MusicItems
    {
        set => ViewModel.UpdateMusicItems(value);
    }

    public MusicListBarViewModel ViewModel { get; }
}