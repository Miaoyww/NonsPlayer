﻿using System.Drawing.Imaging;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using QRCoder;
using System.Collections.ObjectModel;
using FileAttributes = Windows.Storage.FileAttributes;

namespace NonsPlayer.ViewModels;

public partial class LocalViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<LocalFolderModel> Folders = new();
    public LocalPlaylist LocalPlaylist;

    #region test

    public async Task Save()
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
    public async Task Scan(string name)
    {
        FolderPicker openPicker = new Windows.Storage.Pickers.FolderPicker();
        var window = App.MainWindow;
        var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(window);
        WinRT.Interop.InitializeWithWindow.Initialize(openPicker, hWnd);
        openPicker.SuggestedStartLocation = PickerLocationId.Desktop;
        openPicker.FileTypeFilter.Add("*");
        StorageFolder folder = await openPicker.PickSingleFolderAsync();
        LocalPlaylist = new LocalPlaylist(folder.Path, name);
        LocalPlaylist.Songs = await ScanMusic(await folder.GetItemsAsync());
        LocalPlaylist.Save();
    }
    
    private async Task<List<LocalMusic>> ScanMusic(IReadOnlyList<IStorageItem> items)
    {
        List<LocalMusic> musics = new();
        foreach (var file in items)
        {
            if (file.Attributes.HasFlag(FileAttributes.Directory))
            {
                musics.AddRange(await ScanMusic(await (file as StorageFolder).GetItemsAsync()));
            }
            else if (file.Attributes.HasFlag(FileAttributes.Normal))
            {
                // 判断是否为音乐文件
                //TODO: 优化音乐文件判断
                try
                {
                    var track = TagLib.File.Create(file.Path);
                    musics.Add(new LocalMusic(track));
                }
                catch (Exception e)
                {
                    //igrone
                }
            }
        }
    
        return musics;
    }
    //
    // [RelayCommand]
    // public async Task GetInfo()
    // {
    //     var tasks = LocalPlaylist.Songs.Select(async x => await x.TryGetInfo());
    //     await Task.WhenAll(tasks).ConfigureAwait(false);
    // }
    //
    // [RelayCommand]
    // public async Task MixMode()
    // {
    //     Player.Instance.EnqueueTrack(LocalPlaylist.Songs.ToArray());
    //     
    //     PlayQueue.Instance.AddMusicList(LocalPlaylist.Songs.ToArray());
    // }

    #endregion

    public void OnNavigatedTo(object parameter)
    {
        Folders.Clear();
        Folders.Add(new LocalFolderModel("01", "音乐", @"C:\Users\miaom\Music"));
    }

    public void OnNavigatedFrom()
    {
    }
}