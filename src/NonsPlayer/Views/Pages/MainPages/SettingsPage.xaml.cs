using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Dialogs;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Text.RegularExpressions;
using ArtistSeparator = NonsPlayer.Components.Dialogs.ArtistSeparator;

namespace NonsPlayer.Views.Pages;

public sealed partial class SettingsPage : Page
{
    public SettingsPage()
    {
        ViewModel = App.GetService<SettingsViewModel>();
        InitializeComponent();
        ArtistSepSettingsCard.Header = "ArtistSep".GetLocalized();
        ArtistSepTextBlock.Text = "ArtistSepManage".GetLocalized();

        RefreshInfo();
    }

    public SettingsViewModel ViewModel { get; }
    private readonly ControlService _controlService = App.GetService<ControlService>();
    private readonly IThemeSelectorService _themeSelectorService = App.GetService<IThemeSelectorService>();
    private readonly FileService _fileService = App.GetService<FileService>();

    private void RefreshInfo()
    {
        RefreshAdapterInfo();

        switch (_themeSelectorService.Theme)
        {
            case ElementTheme.Light:
                ThemeComboBox.SelectedIndex = 1;
                break;
            case ElementTheme.Dark:
                ThemeComboBox.SelectedIndex = 2;
                break;
            default:
                ThemeComboBox.SelectedIndex = 0;
                break;
        }

        if (ConfigManager.Instance.Settings.SMTCEnable) SMTCSwitcher.IsOn = true;

        try
        {
            var size = FileService.FormatSize(GetCacheSize());
            CacheCard.Header = string.Format("CacheSize".GetLocalized(), size);
        }
        catch(Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
        
    }

    private long GetCacheSize()
    {
        return FileService.GetDirectorySize(ConfigManager.Instance.Settings.LogPath) +
                               FileService.GetDirectorySize(ConfigManager.Instance.Settings.CachePath);
    }

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
                Tag = adapter.GetMetadata().Slug,
                Command = SetAdapterCommand,
                CommandParameter = adapter.GetMetadata().Slug,
            });
        }
    }

    [RelayCommand]
    private async Task CleanCache()
    {
        var paths = new List<string>
        {
            ConfigManager.Instance.Settings.LogPath,
            ConfigManager.Instance.Settings.CachePath
        };
        var sizeBefore = GetCacheSize();
        foreach (string path in paths)
        {
            try
            {
                _fileService.DeleteFolder(path);
            }
            catch (Exception ex)
            {
                ExceptionService.Instance.Throw(ex);
            }
        }
        var sizeAfter = GetCacheSize();
        var cleaned = FileService.FormatSize(sizeBefore - sizeAfter);
        DialogHelper.Instance.Show(string.Format("CacheCleanedNotification".GetLocalized(), cleaned));
        RefreshInfo();
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

    private void ThemeComboBox_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        var index = ((ComboBox)sender).SelectedIndex;
        switch (index)
        {
            case 0:
                _themeSelectorService.SetTheme(ElementTheme.Default);
                break;
            case 1:
                _themeSelectorService.SetTheme(ElementTheme.Light);
                break;
            case 2:
                _themeSelectorService.SetTheme(ElementTheme.Dark);
                break;
            default:
                _themeSelectorService.SetTheme(ElementTheme.Default);
                break;
        }
    }

    private void SMTCSwitcher_OnToggled(object sender, RoutedEventArgs e)
    {
        var isOn = ((ToggleSwitch)sender).IsOn;
        ConfigManager.Instance.Settings.SMTCEnable = isOn;
        ConfigManager.Instance.Save();
    }
}