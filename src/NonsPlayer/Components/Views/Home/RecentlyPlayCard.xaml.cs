using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class RecentlyPlayCard : UserControl
{
    public RecentlyPlayCard()
    {
        ViewModel = App.GetService<RecentlyPlayCardViewModel>();
        InitializeComponent();
        RecentlyPlayTextBlock.Text = "RecentlyPlay".GetLocalized();
        PlayCounterService = App.GetService<PlayCounterService>();
        PlayCounterService.CounterChanged += OnCounterChanged;
        RefreshInfo();
    }

    public RecentlyPlayCardViewModel ViewModel { get; }
    public PlayCounterService PlayCounterService;


    private void OnCounterChanged()
    {
        DispatcherQueue.TryEnqueue(() => { RefreshInfo(); });
    }

    private void RefreshInfo()
    {
        CardPanel.Children.Clear();
        // ȡ������ŵ����׸�
        var lastFourElements = PlayCounterService.RecentlyMusic
            .Skip(Math.Max(0, PlayCounterService.RecentlyMusic.Count - 10)).ToList();
        if (lastFourElements != null)
        {
            lastFourElements.Reverse();
            for (int i = 0; i < lastFourElements.Count; i++)
            {
                var music = lastFourElements[i];
                CardPanel.Children.Add(new RecentlyPlayItemCard
                {
                    Margin = new(8, 0, 8, 0),
                    Music = music
                });
            }
        }

    }
}