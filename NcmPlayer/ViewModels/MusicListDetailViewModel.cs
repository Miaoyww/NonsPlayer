using System.Collections.ObjectModel;
using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NcmPlayer.Contracts.ViewModels;
using NcmPlayer.Framework.Model;
using NcmPlayer.Views;

namespace NcmPlayer.ViewModels;

public class MusicListDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    private MusicListDetailPage musicdetailPage;
    private long currentId;

    public async Task LoadPlaylistAsync(long id)
    {
        PlayList playList = new();
        await playList.LoadAsync(id);
        musicdetailPage.Name = playList.Name;
        musicdetailPage.Creator = playList.Creator;
        musicdetailPage.CreateTime = playList.CreateTime.ToString();
        musicdetailPage.Description = playList.Description;
        musicdetailPage.MusicsCount = playList.MusicsCount.ToString();
        musicdetailPage.SetCover(new Uri(playList.PicUrl));
        Music[] musics = await playList.InitArtWorkList();
        await musicdetailPage.UpdateMusicsList(musics, playList);
    }

    public void OnNavigatedFrom()
    {
    }

    public async void MusicDetailPageLoaded(object sender, RoutedEventArgs e)
    {
        musicdetailPage = (MusicListDetailPage)sender;
        await LoadPlaylistAsync(currentId);
    }

    public async void OnNavigatedTo(object parameter)
    {
        currentId = (long)parameter;
    }

}