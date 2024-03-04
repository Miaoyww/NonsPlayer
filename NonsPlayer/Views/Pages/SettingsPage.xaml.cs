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

    }

    public SettingsViewModel ViewModel { get; }
    public ImageBrush NonsPlayerIco;
    private ControlService _controlService = App.GetService<ControlService>();

    [RelayCommand]
    private async void Test()
    {
        var dialog = new ContentDialog();

        // XamlRoot must be set in the case of a ContentDialog running in a Desktop app
        dialog.XamlRoot = XamlRoot;
        dialog.Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style;
        dialog.Title = "Test";
        dialog.PrimaryButtonText = "Got it";
        dialog.CloseButtonText = "Cancel";
        dialog.DefaultButton = ContentDialogButton.Primary;
        dialog.Content = "test";

        var result = await dialog.ShowAsync();
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