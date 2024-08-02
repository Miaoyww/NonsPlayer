using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class HitokotoCard : UserControl
{

    public HitokotoCard()
    {
        ViewModel = App.GetService<HitokotoCardViewModel>();
        InitializeComponent();
    }
    public HitokotoCardViewModel ViewModel { get; }

}