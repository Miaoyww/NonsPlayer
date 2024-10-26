using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.AMLL.Components.Views;
using NonsPlayer.AMLL.Helpers;
using NonsPlayer.AMLL.Models;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Helpers;
using Windows.UI;

namespace NonsPlayer.AMLL.Components.ViewModels;

public sealed partial class LyricCardViewModel : ObservableObject
{
    [ObservableProperty] private Thickness margin = new Thickness(0);
    [ObservableProperty] private LyricItemModel lyricModel;
    [ObservableProperty] private int index;

    [ObservableProperty] private SolidColorBrush foreground =
        Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;

    [ObservableProperty] public Visibility transVisibility;

    public LyricCardViewModel()
    {
        LyricHelper.Instance.LyricChanged += OnLyricChanged;
    }

    partial void OnLyricModelChanged(LyricItemModel value)
    {
        TransVisibility = value.Lyric.HasTranslation ? Visibility.Visible : Visibility.Collapsed;
    }

    private void OnLyricChanged(LyricLine lyric)
    {
        try
        {
            if (LyricModel == null) return;
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                if (lyric.Equals(LyricModel.Lyric))
                {
                    Foreground = new SolidColorBrush(Colors.White);
                }
                else
                {
                    Foreground = Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;
                }
            });
        }
        catch
        {
            // igrone just ok
        }
    }
}