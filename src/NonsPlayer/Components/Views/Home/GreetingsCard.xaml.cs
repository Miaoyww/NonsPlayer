using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class GreetingsCard : UserControl
{

    public GreetingsCard()
    {
        ViewModel = App.GetService<GreetingsCardViewModel>();
        InitializeComponent();
    }
    public GreetingsCardViewModel ViewModel { get; }
}