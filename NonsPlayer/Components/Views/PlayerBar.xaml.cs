using System.Windows.Input;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;

namespace NonsPlayer.Components.Views;

public sealed partial class PlayerBar : UserControl
{
    public event EventHandler OnPlayQueueBarOpenHandler;
    public PlayerBarViewModel ViewModel
    {
        get;
    }

    public PlayerBar()
    {
        ViewModel = App.GetService<PlayerBarViewModel>();
        InitializeComponent();
        OpenPlayQueueCommand = new RelayCommand(OpenPlayQueueBar);

    }
    public ICommand OpenPlayQueueCommand;
    public void OpenPlayQueueBar()
    {
        OnPlayQueueBarOpenHandler?.Invoke(this, EventArgs.Empty);
    }
    
}