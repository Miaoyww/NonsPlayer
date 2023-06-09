using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class SettingsPage : Page
{
    public SettingsViewModel ViewModel
    {
        get;
    }

    public SettingsPage()
    {
        ViewModel = App.GetService<SettingsViewModel>();
        InitializeComponent();
    }
}