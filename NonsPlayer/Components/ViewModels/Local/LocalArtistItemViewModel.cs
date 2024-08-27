using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Utils;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using System.Diagnostics;
using Windows.Storage;
using Windows.Storage.Pickers;
using FileAttributes = Windows.Storage.FileAttributes;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class LocalArtistItemViewModel
{
    [ObservableProperty] private string name;
    [ObservableProperty] private string count;

    public void Init(LocalArtist artist)
    {
        Name = artist.Name;
        Count = artist.Songs.Count.ToString();
    }
}