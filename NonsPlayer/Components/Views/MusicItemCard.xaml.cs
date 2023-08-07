using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class MusicItemCard : UserControl
{
    [ObservableProperty] private string index;

    [ObservableProperty] private bool isCoverInit;

    [ObservableProperty] private Music music;


    public MusicItemCard()
    {
        ViewModel = App.GetService<MusicItemCardViewModel>();
        InitializeComponent();
    }

    public MusicItemCardViewModel ViewModel { get; }

    partial void OnMusicChanged(Music music)
    {
        ViewModel.Init(music);
    }

    partial void OnIsCoverInitChanged(bool isCoverInit)
    {
        ViewModel.IsInitCover = isCoverInit;
    }

    partial void OnIndexChanged(string index)
    {
        ViewModel.Index = index;
    }
}