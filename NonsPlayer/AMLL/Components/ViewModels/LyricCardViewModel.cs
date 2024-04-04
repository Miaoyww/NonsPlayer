using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.AMLL.Models;
using NonsPlayer.Core.AMLL.Models;

namespace NonsPlayer.AMLL.Components.ViewModels;

public sealed partial class LyricCardViewModel : ObservableObject
{
    [ObservableProperty] private Thickness margin = new Thickness(0);
    [ObservableProperty] private LyricItemModel lyricModel;
    [ObservableProperty] private int index;

    [ObservableProperty] private SolidColorBrush foreground =
        Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;

    [ObservableProperty] public Visibility transVisibility;

    partial void OnLyricModelChanged(LyricItemModel value)
    {
        TransVisibility = value.Lyric.HaveTranslation ? Visibility.Visible : Visibility.Collapsed;
    }

    private void OnLyricChanged(int i)
    {
        try
        {
            if (i - 1 == Index)
            {
                Foreground = new SolidColorBrush(Colors.White);
            }
            else
            {
                Foreground = Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;
            }
        }
        catch
        {
            // igrone just ok
        }
    }
}