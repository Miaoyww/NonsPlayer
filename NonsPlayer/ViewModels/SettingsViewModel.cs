using System.Reflection;
using System.Runtime.InteropServices;
using Windows.ApplicationModel;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;

namespace NonsPlayer.ViewModels;

public partial class SettingsViewModel : ObservableRecipient
{
    private readonly UpdateService _updateService = App.GetService<UpdateService>();
    private readonly VersionService _versionService = App.GetService<VersionService>();

    [ObservableProperty] public string versionDescription;

    public SettingsViewModel(IThemeSelectorService themeSelectorService)
    {
        VersionDescription = _versionService.CurrentVersionDescription;
    }

    [RelayCommand]
    private async Task Test()
    { 
        // Create a file picker
        var openPicker = new Windows.Storage.Pickers.FileOpenPicker();

        // See the sample code below for how to make the window accessible from the App class.
        var window = App.MainWindow;

        // Retrieve the window handle (HWND) of the current WinUI 3 window.
        var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(window);

        // Initialize the file picker with the window handle (HWND).
        WinRT.Interop.InitializeWithWindow.Initialize(openPicker, hWnd);

        // Set options for your file picker
        openPicker.ViewMode = PickerViewMode.Thumbnail;
        openPicker.FileTypeFilter.Add("*");

        // Open the picker for the user to pick a file
        var file = await openPicker.PickSingleFileAsync();
        await Player.Instance.NewPlay(new LocalMusic(file.Path));
    }

    [RelayCommand]
    public async void CheckUpdate()
    {
        var release =
            await _updateService.CheckUpdateAsync(_versionService.CurrentVersion, RuntimeInformation.OSArchitecture);
        if (release != null) ServiceHelper.NavigationService.NavigateTo(typeof(UpdateViewModel).FullName, release);
    }
    
}