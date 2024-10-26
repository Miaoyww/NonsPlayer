using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Services;
using NonsPlayer.Dialogs;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Text.RegularExpressions;

namespace NonsPlayer.Views.Pages;

public sealed partial class SettingsPage : Page
{
    public SettingsPage()
    {
        ViewModel = App.GetService<SettingsViewModel>();
        NonsPlayerIco = new ImageBrush { ImageSource = new BitmapImage(new Uri("ms-appx:///Assets/NonsPlayer.png")) };
        InitializeComponent();
        RefreshAdapterInfo();
        ArtistSepSettingsCard.Header = "ArtistSep".GetLocalized();
        ArtistSepTextBlock.Text = "ArtistSepManage".GetLocalized();
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
    public async Task OpenArtist()
    {
        ContentDialog dialog = new ContentDialog();

        // XamlRoot must be set in the case of a ContentDialog running in a Desktop app
        dialog.XamlRoot = this.XamlRoot;
        dialog.Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style;
        dialog.Title = "ArtistSepManage".GetLocalized();
        dialog.PrimaryButtonText = "Save".GetLocalized();
        dialog.CloseButtonText = "Cancel".GetLocalized();
        dialog.DefaultButton = ContentDialogButton.Primary;
        dialog.Content = new ArtistSeparator();
        var result = await dialog.ShowAsync();

        if (result == ContentDialogResult.Primary)
        {
            var models = (dialog.Content as Page).Tag as ObservableCollection<SeparatorModel>;
            if (models != null)
            {
                ConfigManager.Instance.Settings.LocalArtistSep.Clear();
                foreach (var artistSeparator in models)
                {
                    ConfigManager.Instance.Settings.LocalArtistSep.Add(artistSeparator.Text);
                }
                ConfigManager.Instance.Save();
            }

        }
    }

    [RelayCommand]
    private void SetAdapter(string name)
    {
        ConfigManager.Instance.Settings.DefaultAdapter = name;
        ConfigManager.Instance.Save();
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

    private void OnPreviewKeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (!IsTextAllowed(((NumberBox)sender).Text))
        {
            ((NumberBox)sender).Text = "0";
            e.Handled = true;
        }

    }

    private static bool IsTextAllowed(string text)
    {
        var regex = new Regex(@"^[0-9]\d*$");
        return regex.IsMatch(text);
    }
}