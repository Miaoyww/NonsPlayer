using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using System.Collections.ObjectModel;

namespace NonsPlayer.Components.Dialogs;

[INotifyPropertyChanged]
public sealed partial class ArtistSeparator : Page
{
    public ObservableCollection<SeparatorModel> Models = new();

    public ArtistSeparator()
    {
        InitializeComponent();
        Init();
        AddNewButton.Content = "AddNew".GetLocalized();
        this.Tag = Models;
    }

    private void Init()
    {
        Models.Clear();
        foreach (string s in ConfigManager.Instance.Settings.LocalArtistSep)
        {
            Models.Add(new SeparatorModel { Text = s, Command = DelSepCommand });
        }
    }

    [RelayCommand]
    public void DelSep(string content)
    {
        var itemsToRemove = new List<SeparatorModel>();
        foreach (SeparatorModel separatorModel in Models)
        {
            if (separatorModel.Text.Equals(content))
            {
                itemsToRemove.Add(separatorModel);
            }
        }

        foreach (var item in itemsToRemove)
        {
            Models.Remove(item);
        }
    }

    [RelayCommand]
    public void Add()
    {
        Models.Add(new SeparatorModel { Command = DelSepCommand });
    }
}

[INotifyPropertyChanged]
public partial class SeparatorModel
{
    [ObservableProperty] private string text;
    public IRelayCommand Command;
}