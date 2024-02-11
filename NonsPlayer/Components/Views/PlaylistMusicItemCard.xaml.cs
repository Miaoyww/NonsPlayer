using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PlaylistMusicItemCard : UserControl
{
    [ObservableProperty] private string index;

    [ObservableProperty] private bool isCoverInit;

    [ObservableProperty] private Music music;


    public PlaylistMusicItemCard()
    {
        ViewModel = App.GetService<PlaylistMusicItemCardViewModel>();
        InitializeComponent();
    }

    public PlaylistMusicItemCardViewModel ViewModel { get; }

    partial void OnMusicChanged(Music music)
    {
        ViewModel.Init(music);
        for (int i = 0; i < Music.Artists.Length; i++)
        {
            CheckArtists.Items.Add(new MenuFlyoutItem()
            {
                Text = Music.Artists[i].Name,
                Command = CheckArtistCommand,
                CommandParameter = Music.Artists[i]
            });
        }
    }

    [RelayCommand]
    public void CheckArtist(Artist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    partial void OnIsCoverInitChanged(bool isCoverInit)
    {
        ViewModel.IsInitCover = isCoverInit;
    }

    partial void OnIndexChanged(string index)
    {
        ViewModel.Index = index;
    }

    [RelayCommand]
    private async void Like()
    {
        var code = await FavoritePlaylistService.Instance.Like(Music.Id);
        if (code != 200)
        {
            string content;
            switch (code)
            {
                case 301:
                    content = "请登录后再试";
                    break;
                case 400:
                    content = "请检查网络后再试";
                    break;
                default:
                    content = $"出现了错误 {code}";
                    break;
            }

            var dialog = new ContentDialog
            {
                XamlRoot = XamlRoot,
                Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style,
                Title = "错误",
                PrimaryButtonText = "知道了",
                CloseButtonText = "取消",
                DefaultButton = ContentDialogButton.Primary,
                Content = content
            };
            await dialog.ShowAsync();
        }
    }
}