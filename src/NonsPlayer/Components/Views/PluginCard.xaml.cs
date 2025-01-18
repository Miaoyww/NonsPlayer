using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Helpers;
using NonsPlayer.Models.Github;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class PluginCard : UserControl
{

    public PluginCard()
    {
        ViewModel = App.GetService<PluginCardViewModel>();
        InitializeComponent();
        PluginTypeTextBlock.Text = "Adapter".GetLocalized();
        FunctionButton.Text = "Install".GetLocalized();
        InstalledButton.Text = "Installed".GetLocalized();
        UpdateButton.Text = "Update".GetLocalized();
    }
    public PluginCardViewModel ViewModel { get; }

    public PluginModel PluginModel
    {
        get => ViewModel.Metadata;
        set => ViewModel.Metadata = value;
    }
}