using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class SettingsPage : Page
{
    public SettingsPage()
    {
        ViewModel = App.GetService<SettingsViewModel>();
        NonsPlayerIco = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri("ms-appx:///Assets/NonsPlayer.png"))
        };
        InitializeComponent();
        RefreshAdapterInfo();
    }

    public SettingsViewModel ViewModel { get; }
    public ImageBrush NonsPlayerIco;
    private ControlService _controlService = App.GetService<ControlService>();

    private void RefreshAdapterInfo()
    {
        AdaptersItems.Items.Clear();
        if (!string.IsNullOrEmpty(ConfigManager.Instance.Settings.DefaultAdapter))
        {
            var defaultAdapter =
                AdapterService.Instance.GetAdapter(ConfigManager.Instance.Settings.DefaultAdapter);
            if (defaultAdapter != null)
            {
                AdapterDrop.Content = defaultAdapter.GetMetadata().DisplayPlatform;
            }
        }

        var adapters = AdapterService.Instance.GetLoadedAdapters();
        foreach (var adapter in adapters)
        {
            AdaptersItems.Items.Add(new MenuFlyoutItem
            {
                Text = adapter.GetMetadata().DisplayPlatform,
                Tag = adapter.GetMetadata().Name,
                Command = SetAdapterCommand,
                CommandParameter = adapter.GetMetadata().Name,
            });
        }
    }

    [RelayCommand]
    private void SetAdapter(string name)
    {
        ConfigManager.Instance.Settings.DefaultAdapter = name;
        ConfigManager.Instance.SaveConfig();
        RefreshAdapterInfo();
    }

    private async void ApiSwitch(object sender, RoutedEventArgs e)
    {
        if (((ToggleSwitch)sender).IsOn)
        {
            await _controlService.StartAsync();
        }
        else
        {
            _controlService.Stop();
        }

        await Task.Delay(1000);
    }
}