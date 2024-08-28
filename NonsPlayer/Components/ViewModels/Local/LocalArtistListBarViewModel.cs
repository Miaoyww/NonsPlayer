using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Models;
using System.Collections.ObjectModel;


namespace NonsPlayer.Components.ViewModels;

public partial class LocalArtistListBarViewModel
{
    public ObservableCollection<LocalArtistModel> Models;

    public void Init(ObservableCollection<LocalArtistModel> models)
    {
        Models = models;
    }
}