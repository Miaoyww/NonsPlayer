using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class RecentlyPlayCard : UserControl
{

    public RecentlyPlayCard()
    {
        ViewModel = App.GetService<RecentlyPlayCardViewModel>();
        InitializeComponent();
    }
    public RecentlyPlayCardViewModel ViewModel { get; }

}