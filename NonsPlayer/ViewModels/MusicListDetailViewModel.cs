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

public partial class MusicListDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private Playlist playListObject;
    [ObservableProperty] private string createTime;
    [ObservableProperty] private string creator;
    [ObservableProperty] private long currentId;
    [ObservableProperty] private string description;
    [ObservableProperty] private string musicsCount;
    [ObservableProperty] private string name;

    public MusicListDetailViewModel()
    {
        PlayAllCommand = new RelayCommand(PlayAll);
    }

    public ObservableCollection<MusicItem> MusicItems = new();
    public List<Music> Musics = new();

    public void OnNavigatedFrom()
    {
    }

    public void OnNavigatedTo(object parameter) => currentId = (long)parameter;

    private async Task LoadPlaylistDetailAsync()
    {
        await Task.Run(() =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Name = playListObject.Name;
                Creator = "made by " + playListObject.Creator;
                CreateTime = $"· {playListObject.CreateTime.ToString().Split(" ")[0]}";
                Description = playListObject.Description;
                MusicsCount = playListObject.MusicsCount + "Tracks";
                Cover = CacheHelper.GetImageBrush(playListObject.CacheCoverId, playListObject.CoverUrl);
            });
        });
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

    public async void PageLoaded(object sender, RoutedEventArgs e)
    {
        playListObject = CacheHelper.GetPlaylist(currentId + "_playlist".ToString(), currentId.ToString());
        var playlistLoadedTime = await Tools.MeasureExecutionTimeAsync(playListObject.LoadAsync(currentId));
        Debug.WriteLine($"加载歌单({playListObject.Id})所用时间: {playlistLoadedTime.Milliseconds}ms");
        await Task.WhenAll(LoadPlaylistDetailAsync(), LoadMusicsAsync());
    }


    public ICommand PlayAllCommand
    {
        get;
    }

    public void PlayAll()
    {
        PlayQueue.Instance.AddMusicList(Musics);
    }
}