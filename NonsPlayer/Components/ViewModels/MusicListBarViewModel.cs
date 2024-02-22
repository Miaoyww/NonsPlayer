using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;

namespace NonsPlayer.Components.ViewModels;

public class MusicListBarViewModel : ObservableObject
{
    public ObservableCollection<MusicItem> MusicItems = new();

    public void UpdateMusicItems(ObservableCollection<MusicItem> items)
    {
        MusicItems = items;
    }
}