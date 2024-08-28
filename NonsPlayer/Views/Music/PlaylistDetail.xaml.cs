using Windows.ApplicationModel.DataTransfer;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.Storage.Streams;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using WinRT.Interop;

namespace NonsPlayer.Views;

public sealed partial class PlaylistDetailPage : Page
{
    public PlaylistDetailPage()
    {
        ViewModel = App.GetService<PlaylistDetailViewModel>();
        InitializeComponent();
    }

    public PlaylistDetailViewModel ViewModel { get; }


    [RelayCommand]
    public async void SaveCover()
    {
        var savePicker = new FileSavePicker();
        var window = App.MainWindow;
        var hWnd = WindowNative.GetWindowHandle(window);
        InitializeWithWindow.Initialize(savePicker, hWnd);
        savePicker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        savePicker.FileTypeChoices.Add("图像文件", new List<string> { ".jpg", ".png" });
        savePicker.SuggestedFileName = ViewModel.CurrentId.ToString();
        var file = await savePicker.PickSaveFileAsync();
        if (file != null)
        {
            CachedFileManager.DeferUpdates(file);
            using (var stream = await file.OpenStreamForWriteAsync())
            {
                using (var tw = new StreamWriter(stream))
                {
                    var content = (await ImageHelpers.GetImageStreamFromServer(ViewModel.PlayList.AvatarUrl))
                        .AsStream();
                    var btArray = new byte[512];
                    var contentSize = await content.ReadAsync(btArray, 0, btArray.Length);

                    while (contentSize > 0)
                    {
                        stream.Write(btArray, 0, contentSize);
                        contentSize = await content.ReadAsync(btArray, 0, btArray.Length);
                    }
                }
            }
        }
    }

    [RelayCommand]
    public void CopyCover()
    {
        var data = new DataPackage();
        data.SetBitmap(RandomAccessStreamReference.CreateFromUri(new Uri(ViewModel.PlayList.AvatarUrl)));
        Clipboard.SetContent(data);
    }

    [RelayCommand]
    public void CopyId()
    {
        var data = new DataPackage();
        data.SetText(ViewModel.PlayList.Id.ToString());
        Clipboard.SetContent(data);
    }


    [RelayCommand]
    public void CopyLink()
    {
        var data = new DataPackage();
        data.SetText($"https://music.163.com/playlist?id={ViewModel.PlayList.Id}");
        Clipboard.SetContent(data);
    }


    [RelayCommand]
    public void CopyCreator()
    {
        var data = new DataPackage();
        data.SetText(ViewModel.PlayList.Creator);
        Clipboard.SetContent(data);
    }
}