﻿using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PlayQueueItemCard : UserControl
{
    [ObservableProperty] private string artists;
    [ObservableProperty] private ImageBrush cover;

    /// <summary>
    /// Item1: CacheId Item2: Url Item3: LocalCover
    /// </summary>
    [ObservableProperty] private Tuple<string, string, byte[]> coverUrl;

    [ObservableProperty] private Brush fontBrush = (Brush)Application.Current.Resources["TextFillColorPrimaryBrush"];
    [ObservableProperty] private long id;
    [ObservableProperty] private bool liked; //TODO: Implement this
    [ObservableProperty] private IMusic music;
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;

    public PlayQueueItemCard()
    {
        ViewModel = App.GetService<PlayQueueItemCardViewModel>();
        PlayQueue.Instance.CurrentMusicChanged += OnCurrentMusicChanged;
        InitializeComponent();
    }

    public PlayQueueItemCardViewModel ViewModel { get; }

    async partial void OnCoverUrlChanged(Tuple<string, string, byte[]> value)
    {
        if (value.Item3 != null)
        {
            Cover = await CacheHelper.GetImageBrush(value.Item1, value.Item3);
        }


        Cover = await CacheHelper.GetImageBrushAsync(value.Item1, value.Item2);
    }

    private void OnCurrentMusicChanged(IMusic value)
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


    private void Play(object sender, DoubleTappedRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(Music);
    }
}