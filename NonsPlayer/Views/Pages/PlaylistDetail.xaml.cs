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

    private void ShowMenu(bool isTransient, CommandBarFlyout flyout, UIElement place)
    {
        var myOption = new FlyoutShowOptions();
        myOption.ShowMode = isTransient ? FlyoutShowMode.Transient : FlyoutShowMode.Standard;
        flyout.ShowAt(place, myOption);
    }

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
                    var content = (await CacheHelper.GetImageStreamFromServer(ViewModel.PlayListObject.AvatarUrl))
                        .AsStream();
                    var btArray = new byte[512]; // 定义一个字节数据,用来向readStream读取内容和向writeStream写入内容
                    var contentSize = await content.ReadAsync(btArray, 0, btArray.Length); // 向远程文件读第一次

                    while (contentSize > 0) // 如果读取长度大于零则继续读
                    {
                        stream.Write(btArray, 0, contentSize); // 写入本地文件
                        contentSize = await content.ReadAsync(btArray, 0, btArray.Length); // 继续向远程文件读取
                    }
                }
            }
        }
    }

    [RelayCommand]
    public void CopyCover()
    {
        var data = new DataPackage();
        data.SetBitmap(RandomAccessStreamReference.CreateFromUri(new Uri(ViewModel.PlayListObject.AvatarUrl)));
        Clipboard.SetContent(data);
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