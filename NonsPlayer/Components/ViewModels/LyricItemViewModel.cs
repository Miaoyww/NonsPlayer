using System.Drawing;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class LyricItemViewModel : ObservableObject
{
    [ObservableProperty] private Thickness margin = new Thickness(0);
    [ObservableProperty] private SongLyric songLyric;
    [ObservableProperty] private int index;

    [ObservableProperty] private SolidColorBrush foreground =
        Application.Current.Resources["TextFillColorTertiaryBrush"] as SolidColorBrush;

    public LyricItemViewModel()
    {
        UiHelper.Instance.LyricChanged += OnLyricChanged;
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