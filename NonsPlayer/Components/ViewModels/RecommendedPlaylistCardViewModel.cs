using System.Diagnostics;
using Windows.ApplicationModel.DataTransfer;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class RecommendedPlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    public IPlaylist CurrentPlaylist;
    [ObservableProperty] private string id;
    [ObservableProperty] private string title;

    public void Init(IPlaylist item)
    {
        CurrentPlaylist = item;
        Id = item.Id.ToString();
        Title = item.Name;
        Cover = CacheHelper.GetImageBrush(item.CacheMiddleAvatarId, item.MiddleAvatarUrl);
    }

    [RelayCommand]
    public void CopyShareUrl()
    {
        var data = new DataPackage();
        data.SetText(CurrentPlaylist.ShareUrl);
        Clipboard.SetContent(data);
    }

    [RelayCommand]
    public async void PlayNext()
    {
        // if (CurrentPlaylist.IsCardMode)
        // {
        //     // var elapsed = await Tools.MeasureExecutionTimeAsync(CurrentPlaylist.LoadAsync(CurrentPlaylist.Id))
        //         // .ConfigureAwait(false);
        //     // Debug.WriteLine($"获取歌单Api耗时{elapsed.TotalMilliseconds}ms");
        //     CurrentPlaylist.IsCardMode = false;
        // }

        CurrentPlaylist.InitializeMusics();
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            PlayQueue.Instance.AddNext(CurrentPlaylist.Musics.ToArray());
        });
    }


    public void OpenMusicListDetail(object sender, DoubleTappedRoutedEventArgs e)
    {
        PlaylistHelper.OpenMusicListDetail(CurrentPlaylist, ServiceHelper.NavigationService);
    }
}