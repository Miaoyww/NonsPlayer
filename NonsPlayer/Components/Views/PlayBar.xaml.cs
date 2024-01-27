using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.Views;

public sealed partial class PlayBar : UserControl
{
    public PlayBar()
    {
        ViewModel = App.GetService<PlayerBarViewModel>();
        InitializeComponent();
    }

    public PlayerBarViewModel ViewModel { get; }

    public event EventHandler OnPlayQueueBarOpenHandler;

    [RelayCommand]
    public void OpenLyric()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(LyricViewModel)?.FullName);
    }

    [RelayCommand]
    public void OpenPlayQueueBar()
    {
        OnPlayQueueBarOpenHandler?.Invoke(this, EventArgs.Empty);
    }

    [RelayCommand]
    private async void LikeMusic()
    {
        if (MusicStateModel.Instance.CurrentMusic.IsEmpty) return;
        var code = await FavoritePlaylistService.Instance.Like(MusicStateModel.Instance.CurrentMusic.Id);
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