using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Services;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicItemCardViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public MusicItemCardViewModel()
    {
    }

    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    public Music Music;
    private string _name;
    private string _artists;
    private string _time;
    private ImageBrush _cover;
    private string _album;
    private bool _liked;
    private string _index;

    public bool Liked
    {
        get => _liked;
        set
        {
            _liked = value;
            OnPropertyChanged(nameof(Liked));
        }
    }

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

    public async void Init(Music one)
    {
        Music = one;
        await Task.Run(() =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Cover = new ImageBrush
                {
                    ImageSource = new BitmapImage(new Uri(Music.CoverUrl + "?param=40y40"))
                };
                Name = Music.Name;
                Time = Music.DuartionTimeString;
                Album = Music.AlbumName;
                Artists = string.IsNullOrEmpty(Music.ArtistsName) ? "未知艺人" : Music.ArtistsName;
                Liked = UserPlaylistHelper.Instance.IsLiked(Music.Id);
            });
        });
    }


    public void Play(object sender, PointerRoutedEventArgs e)
    {
        Player.Instance.NewPlay(Music);
    }
}