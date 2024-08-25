using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using Windows.UI.Core;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LocalNewFolderCard : UserControl
{
    private LocalService localService = App.GetService<LocalService>();

    public LocalNewFolderCard()
    {
        ViewModel = App.GetService<LocalNewFolderCardViewModel>();
        ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
        InitializeComponent();
        AddNewFolderTextBlock.Text = "AddFolder".GetLocalized();
    }

    public LocalNewFolderCardViewModel ViewModel { get; }


    private async void UIElement_OnTapped(object sender, TappedRoutedEventArgs e)
    {
        var path = await PickFolder();
        if (string.IsNullOrEmpty(path)) return;
        localService.TryAddDirection(path);
    }

    private async Task<string> PickFolder()
    {
        var openPicker = new FolderPicker();
        var window = App.MainWindow;
        var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(window);
        WinRT.Interop.InitializeWithWindow.Initialize(openPicker, hWnd);
        openPicker.SuggestedStartLocation = PickerLocationId.Desktop;
        openPicker.FileTypeFilter.Add("*");

        // Open the picker for the user to pick a folder
        StorageFolder folder = await openPicker.PickSingleFolderAsync();
        if (folder != null)
        {
            return folder.Path;
        }

        return string.Empty;
    }
}