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
public partial class PlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    public IPlaylist CurrentPlaylist;
    [ObservableProperty] private string id;
    [ObservableProperty] private string title;
    [ObservableProperty] private string creator;
    public UiHelper UiHelper = UiHelper.Instance;

    public void Init(IPlaylist item)
    {
        CurrentPlaylist = item;
        Id = item.Id.ToString();
        Title = item.Name;
        Cover = CacheHelper.GetImageBrush(item.CacheMiddleAvatarId, item.MiddleAvatarUrl);
        Creator = item.Creator;
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
        if (CurrentPlaylist.MusicTrackIds == null)
        {
            await CurrentPlaylist.InitializePlaylist();
        }

        await CurrentPlaylist.InitializeMusics();
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            PlayQueue.Instance.AddNext(CurrentPlaylist.Musics.ToArray());
        });
    }

    public void UIElement_OnTapped(object sender, TappedRoutedEventArgs e)
    {
        PlaylistHelper.OpenMusicListDetail(CurrentPlaylist, ServiceHelper.NavigationService);
    }
}