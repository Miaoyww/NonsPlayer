using System.Collections.ObjectModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class RecommendedPlaylistCardViewModel : ObservableObject
{
    [ObservableProperty] private ImageSource cover;
    [ObservableProperty] private Brush fontColor;
    [ObservableProperty] private Visibility tipVisibility = Visibility.Collapsed;

    public async void Init(IMusic[] music)
    {
        if (music != null)
        {
            var firstMusic = music[0];
            Cover = (await CacheHelper.GetImageBrushAsync(firstMusic.CacheAvatarId,
                firstMusic.GetCoverUrl())).ImageSource;
            TipVisibility = Visibility.Collapsed;
            FontColor = App.Current.Resources["LightTextColor"] as SolidColorBrush;
            return;
        }

        TipVisibility = Visibility.Visible;
        FontColor = App.Current.Resources["CommonTextColor"] as SolidColorBrush;
    }
}