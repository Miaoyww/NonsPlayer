using System.Drawing;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class LyricItemViewModel : ObservableObject
{
    [ObservableProperty] private Visibility transVisibility = Visibility.Visible;
    [ObservableProperty] private ILyricLine? pureLyric;
    [ObservableProperty] private string? transLyric;
    [ObservableProperty] private Thickness margin = new Thickness(0);
    public LyricItemViewModel()
    {
    }

}