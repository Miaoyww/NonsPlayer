using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Utils;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using System.Diagnostics;
using Windows.Storage;
using Windows.Storage.Pickers;
using FileAttributes = Windows.Storage.FileAttributes;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class LocalFolderItemViewModel
{
    [ObservableProperty] private string index;
    [ObservableProperty] private string path;
    [ObservableProperty] private string count;
    private LocalService localService = App.GetService<LocalService>();

    async partial void OnPathChanged(string value)
    {
        var result = await localService.ScanMusic(value);
        localService.AddSongs(result);
        Count = string.Format("SongCount".GetLocalized(), result.Count);
    }

    [RelayCommand]
    public void OpenFolder()
    {
        Process.Start("explorer.exe", Path);
    }

    [RelayCommand]
    public void DelFolder()
    {
        localService.TryDelDirection(Path);
    }
}