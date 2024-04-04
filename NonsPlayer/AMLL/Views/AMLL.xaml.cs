using Microsoft.UI.Xaml.Controls;
using NonsPlayer.AMLL.ViewModels;

namespace NonsPlayer.AMLL.Views;

public sealed partial class AMLL : UserControl
{
    public AMLLViewModel ViewModel
    {
        get;
    }

    public AMLL()
    {
        ViewModel = App.GetService<AMLLViewModel>();
        InitializeComponent();
    }
}