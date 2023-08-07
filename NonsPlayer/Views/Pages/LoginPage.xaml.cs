using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class LoginPage : Page
{
    public LoginPage()
    {
        ViewModel = App.GetService<LoginViewModel>();
        InitializeComponent();
        ViewModel.Init();
    }

    public LoginViewModel ViewModel { get; }
}