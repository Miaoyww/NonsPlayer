using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

[INotifyPropertyChanged]
public sealed partial class LoginPage : Page
{
    [ObservableProperty] private bool previousButtonEnable;

    public LoginPage()
    {
        ViewModel = App.GetService<LoginViewModel>();
        InitializeComponent();
        CheckButtonEnable();
    }

    public LoginViewModel ViewModel { get; }

    private void NextButton_Clicked(object sender, RoutedEventArgs e)
    {
        LoginCardView.SelectedIndex = 1;
        if (LoginCardView.SelectedIndex == 1) PreviousButtonEnable = true;
    }

    private void PreviousButton_Clicked(object sender, RoutedEventArgs e)
    {
        LoginCardView.SelectedIndex = 0;
        if (LoginCardView.SelectedIndex == 0) PreviousButtonEnable = false;

    }

    private void LoginCardView_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        CheckButtonEnable();
    }

    private void CheckButtonEnable()
    {
        if (LoginCardView.SelectedIndex == 0) PreviousButtonEnable = false;
        if (LoginCardView.SelectedIndex == 1) PreviousButtonEnable = true;
    }
}