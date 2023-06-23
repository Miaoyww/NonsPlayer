using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Framework.Model;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicItemCardViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public MusicItemCardViewModel()
    {
    }

    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    private Music _music;
    private string _name;
    private string _artists;
    private string _time;
    private ImageBrush _cover;
    private string _album;
    private string _index;

    public string Index
    {
        get => _index;
        set
        {
            _index = value;
            OnPropertyChanged(nameof(Index));
        }
    }

    public string Album
    {
        get => _album;
        set
        {
            _album = value;
            OnPropertyChanged(nameof(Album));
        }
    }

    public string Name
    {
        get => _name;
        set
        {
            _name = value;
            OnPropertyChanged(nameof(Name));
        }
    }

    public string Artists
    {
        get => _artists;
        set
        {
            _artists = value;
            OnPropertyChanged(nameof(Artists));
        }
    }

    public string Time
    {
        get => _time;
        set
        {
            _time = value;
            OnPropertyChanged(nameof(Time));
        }
    }

    public ImageBrush Cover
    {
        get => _cover;
        set
        {
            _cover = value;
            OnPropertyChanged(nameof(Cover));
        }
    }

    public void Init(Music one)
    {
        _music = one;
        Cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(one.CoverUrl + "?param=40y40"))
        };
        Name = one.Name;
        Time = one.DuartionTimeString;
        Album = one.AlbumName;
        Artists = string.IsNullOrEmpty(one.ArtistsName) ? "未知艺人" : one.ArtistsName;
    }

    public void Play(object sender, PointerRoutedEventArgs e)
    {
        MusicPlayerHelper.Player.NewPlay(_music);
    }
}