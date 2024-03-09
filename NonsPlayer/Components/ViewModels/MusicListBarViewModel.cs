using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.Models;

namespace NonsPlayer.Components.ViewModels;

public class MusicListBarViewModel : ObservableObject
{
    public ObservableCollection<MusicModel> MusicItems = new();

    public void UpdateMusicItems(ObservableCollection<MusicModel> items)
    {
        MusicItems = items;
    }
}