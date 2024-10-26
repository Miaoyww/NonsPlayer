using System.Collections.ObjectModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

public sealed partial class MusicListBar : UserControl
{
    public MusicListBar()
    {
        ViewModel = App.GetService<MusicListBarViewModel>();
        InitializeComponent();
        TitleTextBlock.Text = "Title".GetLocalized();
    }

    public ObservableCollection<MusicModel> MusicItems
    {
        set => ViewModel.UpdateMusicItems(value);
    }

    public MusicListBarViewModel ViewModel { get; }
}