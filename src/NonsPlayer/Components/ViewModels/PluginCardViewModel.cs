using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models.Github;
using RestSharp;
using RestSharp.Extensions;
using System.Diagnostics;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class PluginCardViewModel
{
    [ObservableProperty] private ImageSource preview;
    [ObservableProperty] private string name;
    [ObservableProperty] private string platform;
    [ObservableProperty] private string author;
    [ObservableProperty] private string description;
    [ObservableProperty] private string version;
    [ObservableProperty] private string buildTime;
    [ObservableProperty] private string stars;
    [ObservableProperty] private PluginModel metadata;

    [ObservableProperty] private bool installed;
    [ObservableProperty] private Visibility visUninstalled = Visibility.Visible;
    [ObservableProperty] private Visibility visUpgrade = Visibility.Collapsed;
    [ObservableProperty] private Visibility visInstalled = Visibility.Collapsed;

    [ObservableProperty] private bool downloadButtonEnable = true;

    partial void OnMetadataChanged(PluginModel value)
    {
        Name = value.Name;
        Author = value.Author;
        BuildTime = value.UpdateTime?.ToString("d");
        Task.Run(async () =>
        {
            if (!value.IsInitialized)
            {
                await value.GetManifestAsync();
                value.IsInitialized = true;
            }

            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Description = value.Description;
                Version = value.Version.ToString();
                Preview = new BitmapImage(new Uri(value.Preview));

                foreach (IAdapter loadedAdapter in AdapterService.Instance.GetLoadedAdapters())
                {
                    if (metadata.Slug.Equals(loadedAdapter.GetMetadata().Slug))
                    {
                        Installed = true;
                        VisUninstalled = Visibility.Collapsed;
                        if (metadata.Version.CompareTo(loadedAdapter.GetMetadata().Version) < 1)
                        {
                            VisUpgrade = Visibility.Visible;
                            break;
                        }

                        VisInstalled = Visibility.Visible;
                        break;
                    }

                    Installed = false;
                }
            });
        });
        Task.Run(async () =>
        {
            if (value.Stars == null)
            {
                await value.GetRepoInfoAsync();
            }

            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Author = value.Author;
                Stars = value.Stars.ToString();
            });
        });
    }

    public void GetInstalled()
    {
        Installed = true;
        VisUninstalled = Visibility.Collapsed;
        VisUpgrade = Visibility.Collapsed;
        VisInstalled = Visibility.Visible;
    }

    [RelayCommand]
    public void OpenFolder()
    {
        Process.Start(new ProcessStartInfo
        {
            FileName = ConfigManager.Instance.Settings.AdapterPath, UseShellExecute = true
        });
    }

    [RelayCommand]
    public void Delete()
    {
        AdapterService.Instance.DisableAdapter(metadata.Slug);
    }

    [RelayCommand]
    public void OpenGithub()
    {
        Process.Start(new ProcessStartInfo
        {
            FileName = Metadata.RepoUrl.ToString(), UseShellExecute = true // 必须设置为 true 才能打开 URL
        });
    }

    [RelayCommand]
    public void Install()
    {
        var asset = metadata.Release.Assets[0];
        var file = Path.Combine(ConfigManager.Instance.Settings.AdapterPath, asset.Name);

        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            DownloadButtonEnable = false;
        });
        Task.Run(async () =>
        {
            await DownloadFileAsync(asset.BrowserDownloadUrl, file);
            AdapterService.Instance.LoadAdapters(ConfigManager.Instance.Settings.AdapterPath);
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                GetInstalled();
            });
        });
    }

    [RelayCommand]
    public async void Upgrade()
    {
        DownloadButtonEnable = false;
    }


    private async Task DownloadFileAsync(string url, string destinationPath)
    {
        var client = new RestClient();
        var request = new RestRequest(url, Method.GET);

        // 获取响应流
        var response = client.DownloadData(request);
        if (response == null)
        {
            throw new Exception("Failed to download the file.");
        }

        response.SaveAs(destinationPath);
    }
}