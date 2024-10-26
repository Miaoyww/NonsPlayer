using Microsoft.UI.Xaml.Controls;

using NonsPlayer.ViewModels;

namespace NonsPlayer.Views;

public sealed partial class PersonalLibaryPage : Page
{
    public PersonalLibaryViewModel ViewModel
    {
        get;
    }

    public PersonalLibaryPage()
    {
        ViewModel = App.GetService<PersonalLibaryViewModel>();
        InitializeComponent();
    }
}
