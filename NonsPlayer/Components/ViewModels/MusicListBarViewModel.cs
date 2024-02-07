using System.Collections.ObjectModel;
using ABI.Microsoft.UI.Xaml.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using Windows.ApplicationModel.DataTransfer;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicListBarViewModel : ObservableObject
{
    public ObservableCollection<MusicItem> MusicItems = new();

    public MusicListBarViewModel()
    {
    }

    [RelayCommand]
    public void CopyShareUrl()
    {
        var data = new DataPackage();
        data.SetText(MusicItems[0].Music.ShareUrl);
        Clipboard.SetContent(data);
    }

    public void UpdateMusicItems(ObservableCollection<MusicItem> items)
    {
        MusicItems = items;
    }
}