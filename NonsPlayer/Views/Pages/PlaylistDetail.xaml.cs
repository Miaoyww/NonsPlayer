using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Streams;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views;

public sealed partial class PlaylistDetailPage : Page
{
    public PlaylistDetailPage()
    {
        ViewModel = App.GetService<PlaylistDetailViewModel>();
        InitializeComponent();
    }

    private void ShowMenu(bool isTransient, CommandBarFlyout flyout, UIElement place)
    {
        FlyoutShowOptions myOption = new FlyoutShowOptions();
        myOption.ShowMode = isTransient ? FlyoutShowMode.Transient : FlyoutShowMode.Standard;
        flyout.ShowAt(place, myOption);
    }

    public PlaylistDetailViewModel ViewModel { get; }

    private void Cover_OnRightTapped(object sender, RightTappedRoutedEventArgs e)
    {
        ShowMenu(true, CoverFlyout, Cover);
    }

    private void Cover_OnContextRequested(UIElement sender, ContextRequestedEventArgs args)
    {
        ShowMenu(false, CoverFlyout, Cover);
    }

    [RelayCommand]
    public void CopyCover()
    {
        var data = new DataPackage();
        data.SetBitmap(RandomAccessStreamReference.CreateFromUri(new Uri(ViewModel.PlayListObject.AvatarUrl)));
        Clipboard.SetContent(data);
        ShowMenu(false, CoverFlyout, Cover);
    }

    [RelayCommand]
    public void CopyId()
    {
        var data = new DataPackage();
        data.SetText(ViewModel.PlayListObject.Id.ToString());
        Clipboard.SetContent(data);
    }


    [RelayCommand]
    public void CopyLink()
    {
        var data = new DataPackage();
        data.SetText($"https://music.163.com/playlist?id={ViewModel.PlayListObject.Id}");
        Clipboard.SetContent(data);
    }


    [RelayCommand]
    public void CopyCreator()
    {
        var data = new DataPackage();
        data.SetText(ViewModel.PlayListObject.Creator);
        Clipboard.SetContent(data);
    }
}