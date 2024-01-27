using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers;

[INotifyPropertyChanged]
public partial class SearchHelper
{
    public delegate void BestMusicResultChangedHandler(Music value);

    [ObservableProperty] private Music bestMusicResult;
    public static SearchHelper Instance { get; set; } = new();

    public event BestMusicResultChangedHandler BestMusicResultChanged;

    partial void OnBestMusicResultChanged(Music value)
    {
        BestMusicResultChanged(value);
    }
}