using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Cache;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class PlaylistDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private Playlist playListObject;
    [ObservableProperty] private string createTime;
    [ObservableProperty] private string creator;
    [ObservableProperty] private long currentId;
    [ObservableProperty] private string description;
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;

    public ObservableCollection<MusicItem> MusicItems = new();
    public List<Music> Musics = new();

    public void OnNavigatedFrom()
    {
    }

    public async void OnNavigatedTo(object parameter)
    {
        if ((long)parameter == CurrentId)
        {
            return;
        }

        CurrentId = (long)parameter;
        PlayListObject = await CacheHelper.GetPlaylistAsync(CurrentId + "_playlist".ToString(), CurrentId.ToString());
        await Task.WhenAll(LoadPlaylistDetailAsync(), LoadMusicsAsync());
    }

    private async Task LoadPlaylistDetailAsync()
    {
        Name = PlayListObject.Name;
        Creator = "made by " + PlayListObject.Creator;
        CreateTime = $"· {PlayListObject.CreateTime.ToString().Split(" ")[0]}";
        Description = PlayListObject.Description;
        MusicsCount = PlayListObject.MusicsCount + "Tracks";
        Cover = CacheHelper.GetImageBrush(PlayListObject.CacheCoverId, PlayListObject.CoverUrl);
    }

    private async Task LoadMusicsAsync()
    {
        await playListObject.InitMusicsAsync();

        for (var i = 0; i < playListObject.MusicsCount; i++)
        {
            Musics.Add(playListObject.Musics[i]);
            MusicItems.Add(new MusicItem
            {
                Music = playListObject.Musics[i],
                Index = (i + 1).ToString("D2")
            });
        }
    }

    [RelayCommand]
    private void PlayAll() => PlayQueue.Instance.AddMusicList(Musics);
}