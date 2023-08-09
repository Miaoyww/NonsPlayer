using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PlayQueueItemCard : UserControl
{    
    [ObservableProperty] private Brush fontBrush = (Brush) Application.Current.Resources["TextFillColorPrimaryBrush"];

    public PlayQueueItemCard()
    {
        ViewModel = App.GetService<PlayQueueItemCardViewModel>();
        PlayQueue.Instance.CurrentMusicChanged += OnCurrentMusicChanged;
        InitializeComponent();
    }

    public PlayQueueItemCardViewModel ViewModel { get; }

    public Music Music
    {
        set => ViewModel.Music = value;
    }

    public bool IsPlaying
    {
        set{
            if (value)
            {
                FontBrush = (Brush) Application.Current.Resources["AccentFillColorSecondaryBrush"];
            }
        }
    }
    private void OnCurrentMusicChanged(Music value)
    {
        if (value == ViewModel.Music)
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
        PlayQueue.Instance.Play(ViewModel.Music);
    }
}