using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers;

[INotifyPropertyChanged]
public partial class SearchHelper
{
    public delegate void BestMusicResultChangedHandler(IMusic value);

    [ObservableProperty] private IMusic bestMusicResult;
    public static SearchHelper Instance { get; set; } = new();

    public event BestMusicResultChangedHandler BestMusicResultChanged;

    partial void OnBestMusicResultChanged(IMusic value)
    {
        BestMusicResultChanged(value);
    }
}