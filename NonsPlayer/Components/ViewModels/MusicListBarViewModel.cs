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

namespace NonsPlayer.Components.ViewModels;

public partial class MusicListBarViewModel : ObservableObject
{
    public ObservableCollection<MusicItem> MusicItems = new();

    public MusicListBarViewModel()
    {
    }

    public void UpdateMusicItems(ObservableCollection<MusicItem> items)
    {
        MusicItems = items;
    }
    

}