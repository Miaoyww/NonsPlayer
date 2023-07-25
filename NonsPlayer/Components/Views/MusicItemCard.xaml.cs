using System.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Views;

public sealed partial class MusicItemCard : UserControl, INotifyPropertyChanged
{
    public MusicItemCardViewModel ViewModel
    {
        get;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    public MusicItemCard()
    {
        ViewModel = App.GetService<MusicItemCardViewModel>();
        InitializeComponent();
    }

    public Music Music
    {
        set => ViewModel.Init(value);
    }

    public string Index
    {
        set => ViewModel.Index = value;
    }
}