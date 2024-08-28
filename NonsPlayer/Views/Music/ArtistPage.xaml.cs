using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Pickers;
using Windows.Storage.Streams;
using Windows.Storage;
using WinRT.Interop;

namespace NonsPlayer.Views.Pages;

public sealed partial class ArtistPage : Page
{
    public ArtistPage()
    {
        ViewModel = App.GetService<ArtistViewModel>();
        InitializeComponent();
    }

    public ArtistViewModel ViewModel { get; }

    [RelayCommand]
    public async void SaveCover()
    {
        var savePicker = new FileSavePicker();
        var window = App.MainWindow;
        var hWnd = WindowNative.GetWindowHandle(window);
        InitializeWithWindow.Initialize(savePicker, hWnd);
        savePicker.SuggestedStartLocation = PickerLocationId.DocumentsLibrary;
        savePicker.FileTypeChoices.Add("图像文件", new List<string> { ".jpg", ".png" });
        savePicker.SuggestedFileName = ViewModel.Name;
        var file = await savePicker.PickSaveFileAsync();
        if (file != null)
        {
            CachedFileManager.DeferUpdates(file);
            using (var stream = await file.OpenStreamForWriteAsync())
            {
                using (var tw = new StreamWriter(stream))
                {
                    var content = (await ImageHelpers.GetImageStreamFromServer(ViewModel.CurrentArtist.AvatarUrl))
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
        data.SetBitmap(RandomAccessStreamReference.CreateFromUri(new Uri(ViewModel.CurrentArtist.AvatarUrl)));
        Clipboard.SetContent(data);
    }
}