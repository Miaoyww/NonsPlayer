using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NcmPlayer.Contracts.ViewModels;
using NcmPlayer.Framework.Model;
using NcmPlayer.Views;

namespace NcmPlayer.ViewModels;

public class MusicListDetailViewModel : ObservableRecipient, INavigationAware
{
    private MusicListDetailPage musicdetailPage;
    private long currentId;

    public MusicListDetailViewModel()
    {
    }

    public void OnNavigatedFrom()
    {
    }

    public async void MusicDetailPageLoaded(object sender, RoutedEventArgs e)
    {
        musicdetailPage = (MusicListDetailPage)sender;
        UpdateInfo(currentId);
    }

    public async void OnNavigatedTo(object parameter)
    {
        currentId = (long)parameter;
    }

    private async void UpdateInfo(long id)
    {
        PlayList playList = new(id);
        musicdetailPage.Name = playList.Name;
        musicdetailPage.Creator = playList.Creator;
        musicdetailPage.CreateTime = playList.CreateTime.ToString();
        musicdetailPage.Description = playList.Description;
        musicdetailPage.MusicsCount = playList.MusicsCount.ToString();
        musicdetailPage.SetCover(new Uri(playList.PicUrl));
        Music[] musics = playList.InitArtWorkList().Result;
        musicdetailPage.UpdateMusicsList(musics, playList);
    }
}