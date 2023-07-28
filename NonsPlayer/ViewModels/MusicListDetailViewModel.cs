using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public class MusicListDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    private ImageBrush _cover;
    private Playlist _this;
    private string _createTime;
    private string _creator;
    private long _currentId;
    private string _description;
    private string _musicsCount;
    private string _name;

    public MusicListDetailViewModel()
    {
        PlayAllCommand = new RelayCommand(PlayAll);
    }

    public ObservableCollection<MusicItem> MusicItems = new();
    public List<Music> Musics = new();

    public string Name
    {
        set
        {
            _name = value;
            OnPropertyChanged(nameof(Name));
        }
        get => _name;
    }

    public string CreateTime
    {
        set
        {
            _createTime = $"· {value.Split(" ")[0]}";
            OnPropertyChanged(nameof(CreateTime));
        }
        get => _createTime;
    }

    public string MusicsCount
    {
        set
        {
            _musicsCount = $"{value} Tracks";
            OnPropertyChanged(nameof(MusicsCount));
        }
        get => _musicsCount;
    }

    public string Description
    {
        set
        {
            _description = value;
            OnPropertyChanged(nameof(Description));
        }
        get => _description;
    }

    public string Creator
    {
        set
        {
            _creator = $"made by {value}";
            OnPropertyChanged(nameof(Creator));
        }
        get => _creator;
    }

    public ImageBrush Cover
    {
        set
        {
            _cover = value;
            OnPropertyChanged(nameof(Cover));
        }
        get => _cover;
    }

    public void OnNavigatedFrom()
    {
    }

    public void OnNavigatedTo(object parameter) => _currentId = (long)parameter;
    public new event PropertyChangedEventHandler? PropertyChanged;


    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    private async Task LoadPlaylistDetailAsync()
    {
        await Task.Run(() =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Name = _this.Name;
                Creator = _this.Creator;
                CreateTime = _this.CreateTime.ToString();
                Description = _this.Description;
                MusicsCount = _this.MusicsCount.ToString();
                Cover = new ImageBrush
                {
                    ImageSource = new BitmapImage(new Uri(_this.PicUrl))
                };
            });
        });
    }

    private async Task LoadMusicsAsync()
    {
        await _this.InitMusicsAsync();

        for (var i = 0; i < _this.MusicsCount; i++)
        {
            Musics.Add(_this.Musics[i]);
            MusicItems.Add(new MusicItem
            {
                Music = _this.Musics[i],
                Index = (i + 1).ToString("D2")
            });

            OnPropertyChanged(nameof(MusicItems));
        }
    }

    public async void PageLoaded(object sender, RoutedEventArgs e)
    {
        Playlist playlist = new();
        _this = playlist;
        var playlistLoadedTime = await Tools.MeasureExecutionTimeAsync(playlist.LoadAsync(_currentId));
        Debug.WriteLine($"加载歌单({_this.Id})所用时间: {playlistLoadedTime.Milliseconds}ms");
        await Task.WhenAll(LoadPlaylistDetailAsync(), LoadMusicsAsync());
    }


    public ICommand PlayAllCommand
    {
        get;
    }

    public void PlayAll()
    {
        PlayQueueService.Instance.AddMusicList(Musics);
    }
}