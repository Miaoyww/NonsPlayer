using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class PersonalCenterPage : Page
{
    public PersonalCenterPage()
    {
        ViewModel = App.GetService<PersonalCenterViewModel>();
        InitializeComponent();
    }

    public PersonalCenterViewModel ViewModel { get; }
}