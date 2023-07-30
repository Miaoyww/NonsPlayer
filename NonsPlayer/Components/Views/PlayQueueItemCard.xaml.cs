using System.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;

namespace NonsPlayer.Components.Views;

public sealed partial class PlayQueueItemCard : UserControl, INotifyPropertyChanged
{
    public PlayQueueItemCardViewModel ViewModel
    {
        get;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    public PlayQueueItemCard()
    {
        ViewModel = App.GetService<PlayQueueItemCardViewModel>();
        InitializeComponent();
    }

    public Music Music
    {
        set => ViewModel.Music = value;
    }
    
    public void Play(object sender, PointerRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(ViewModel.Music);
    }
}