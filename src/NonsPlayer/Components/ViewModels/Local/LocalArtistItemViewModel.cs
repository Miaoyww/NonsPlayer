using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;


namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class LocalArtistItemViewModel
{
    [ObservableProperty] private LocalArtist artist;
    [ObservableProperty] private string index;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string name;
    [ObservableProperty] private string count;
}