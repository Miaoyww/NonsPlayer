﻿using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Framework.Model;
using NonsPlayer.Framework.Player;

namespace NonsPlayer.ViewModels;

public class MusicListDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    private ImageBrush _cover;
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

    public async Task LoadPlaylistAsync(long id)
    {
        PlayList playList = new();
        await playList.LoadAsync(id);
        Name = playList.Name;
        Creator = playList.Creator;
        CreateTime = playList.CreateTime.ToString();
        Description = playList.Description;
        MusicsCount = playList.MusicsCount.ToString();
        Cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(playList.PicUrl))
        };
        var musics = await playList.InitArtWorkList();
        await UpdateMusicsList(musics, playList);
    }

    public void PageLoaded(object sender, RoutedEventArgs e)
    {
        LoadPlaylistAsync(_currentId);
    }

    public async Task UpdateMusicsList(Music[] musics, PlayList list)
    {
        for (int i = 0; i < musics.Length; i++)
        {
            Musics.Add(musics[i]);
            MusicItems.Add(new MusicItem
            {
                Music = musics[i],
                Index = (i + 1).ToString("D2")
            });
            OnPropertyChanged(nameof(MusicItems));
        }
    }

    public ICommand PlayAllCommand
    {
        get;
    }

    public void PlayAll()
    {
        Playlist.Instance.AddMusicList(Musics);
    }
}