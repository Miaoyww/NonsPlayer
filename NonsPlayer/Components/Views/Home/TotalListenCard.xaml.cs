using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class TotalListenCard : UserControl
{

    public TotalListenCard()
    {
        ViewModel = App.GetService<TotalListenCardViewModel>();
        InitializeComponent();
    }
    public TotalListenCardViewModel ViewModel { get; }

}