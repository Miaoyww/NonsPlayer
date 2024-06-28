using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class AdapterCard : UserControl
{

    public AdapterCard()
    {
        ViewModel = App.GetService<AdapterCardViewModel>();
        InitializeComponent();
    }
    public AdapterCardViewModel ViewModel { get; }

    public AdapterMetadata AdapterMetadata
    {
        get => ViewModel.Metadata;
        set => ViewModel.Metadata = value;
    }

    public int Index
    {
        get => int.Parse(ViewModel.Index);
        set => ViewModel.Index = value.ToString("D2");
    }
}