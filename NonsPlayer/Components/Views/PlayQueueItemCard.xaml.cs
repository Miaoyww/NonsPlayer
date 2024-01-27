using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PlayQueueItemCard : UserControl
{
    [ObservableProperty] private string artists;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private Tuple<string, string> coverUrl;
    [ObservableProperty] private Brush fontBrush = (Brush)Application.Current.Resources["TextFillColorPrimaryBrush"];
    [ObservableProperty] private long id;
    [ObservableProperty] private bool liked; //TODO: Implement this
    [ObservableProperty] private Music music;
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;

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
        // TODO: 这b玩意BUG怎么这么多?
        // if (PlayQueue.Instance.CurrentMusic.Id == Id)
        // {
        //     FontBrush = (Brush) Application.Current.Resources["AccentFillColorSecondaryBrush"];
        // }
        // else
        // {
        //     FontBrush = (Brush) Application.Current.Resources["TextFillColorPrimaryBrush"];
        // }
    }

    public void Play(object sender, PointerRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(Music);
    }
}