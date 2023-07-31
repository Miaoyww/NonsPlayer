using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class MusicItemCard : UserControl
{
    public MusicItemCardViewModel ViewModel
    {
        get;
    }


    public MusicItemCard()
    {
        ViewModel = App.GetService<MusicItemCardViewModel>();
        InitializeComponent();
    }

    [ObservableProperty] private Music music;

    partial void OnMusicChanged(Music music)
    {
        ViewModel.Init(music);
    }

    [ObservableProperty] private bool isCoverInit;

    partial void OnIsCoverInitChanged(bool isCoverInit)
    {
        ViewModel.IsInitCover = isCoverInit;
    }

    [ObservableProperty] private string index;

    partial void OnIndexChanged(string index)
    {
        ViewModel.Index = index;
    }
}