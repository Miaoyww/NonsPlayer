using System.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Framework.Model;

namespace NonsPlayer.Components.Views;

public sealed partial class MusicItemCard : UserControl, INotifyPropertyChanged
{
    public MusicItemCardViewModel ViewModel
    {
        get;
    }

    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    public MusicItemCard()
    {
        ViewModel = App.GetService<MusicItemCardViewModel>();
        InitializeComponent();
    }

    public Music Music
    {
        set
        {
            ViewModel.Init(value);
        }
    }

    public string Index
    {
        set
        {
            ViewModel.Index = value;
        }
    }
}