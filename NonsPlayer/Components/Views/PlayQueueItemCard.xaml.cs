using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PlayQueueItemCard : UserControl
{
    [ObservableProperty] private Brush fontBrush = (Brush) Application.Current.Resources["TextFillColorPrimaryBrush"];
    [ObservableProperty] private string artists;
    [ObservableProperty] private Tuple<string, string> coverUrl;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private bool liked; //TODO: Implement this
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;
    [ObservableProperty] private long id;

    public PlayQueueItemCard()
    {
        ViewModel = App.GetService<PlayQueueItemCardViewModel>();
        PlayQueue.Instance.CurrentMusicChanged += OnCurrentMusicChanged;
        InitializeComponent();
    }

    public PlayQueueItemCardViewModel ViewModel { get; }

    partial void OnCoverUrlChanged(Tuple<string, string> value)
    {
        Cover = CacheHelper.GetImageBrush(value.Item1, value.Item2);
    }

    private void OnCurrentMusicChanged(Music value)
    {
        FontBrushChanger();
    }

    private void PlayQueueItemCard_OnLoaded(object sender, RoutedEventArgs e)
    {
        FontBrushChanger();
    }

    private void FontBrushChanger()
    {
        if (PlayQueue.Instance.CurrentMusic.Id == Id)
        {
            FontBrush = (Brush) Application.Current.Resources["AccentFillColorSecondaryBrush"];
        }
        else
        {
            FontBrush = (Brush) Application.Current.Resources["TextFillColorPrimaryBrush"];
        }
    }

    public void Play(object sender, PointerRoutedEventArgs e)
    {
        App.GetService<PlayQueueBarViewModel>().Play(Id);
    }
}