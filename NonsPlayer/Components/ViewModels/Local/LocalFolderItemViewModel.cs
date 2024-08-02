using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Core.Models;
using System.Diagnostics;
using Windows.Storage;
using Windows.Storage.Pickers;
using FileAttributes = Windows.Storage.FileAttributes;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class LocalFolderItemViewModel
{
    [ObservableProperty] private string index;
    [ObservableProperty] private string name;
    [ObservableProperty] private string path;
    [ObservableProperty] private string count;

    async partial void OnPathChanged(string value)
    {
        var result = await ScanMusic(value);
        Count = result.Count.ToString();
    }

    [RelayCommand]
    public void OpenFolder()
    {
        Process.Start("explorer.exe",Path);
    }

    private async Task<List<LocalMusic>> ScanMusic(string folderPath)
    {
        StorageFolder folder = await StorageFolder.GetFolderFromPathAsync(folderPath);
        var items = await folder.GetItemsAsync();
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

    private async Task<List<LocalMusic>> ScanMusic(IReadOnlyList<IStorageItem> items)
    {
        List<LocalMusic> musics = new();
        foreach (var file in items)
        {
            if (file.Attributes.HasFlag(Windows.Storage.FileAttributes.Directory))
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
}