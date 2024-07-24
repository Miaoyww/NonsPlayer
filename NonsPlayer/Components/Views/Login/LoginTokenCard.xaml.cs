using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LoginTokenCard : UserControl
{
    private IAccount account;
    private IAdapter adapter;
    public LoginTokenCard()
    {
        ViewModel = App.GetService<LoginCardViewModel>();
        InitializeComponent();
    }
    public LoginCardViewModel ViewModel { get; }

    public IAdapter Adapter
    {
        set => adapter = value;
    }
}