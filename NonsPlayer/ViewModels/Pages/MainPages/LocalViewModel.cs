using ATL;
using System.Drawing.Imaging;
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
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class LocalViewModel : ObservableObject, INavigationAware
{
    public LocalService LocalService = App.GetService<LocalService>();
    [ObservableProperty] private string directoriesCount;

    public LocalViewModel()
    {
        LocalService.LocalFolderChanged += LocalServiceOnLocalFolderChanged;
    }

    private void LocalServiceOnLocalFolderChanged(string e)
    {
        RefreshInfo();
    }

    public void RefreshInfo()
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            DirectoriesCount = string.Format("DirectoriesCount".GetLocalized(), LocalService.Directories.Count);
        });
    }

    public void OnNavigatedTo(object parameter)
    {
        RefreshInfo();
    }

    public void OnNavigatedFrom()
    {
    }
}