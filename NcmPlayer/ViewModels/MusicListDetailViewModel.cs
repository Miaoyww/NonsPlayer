using System.ComponentModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NcmPlayer.Contracts.ViewModels;
using NcmPlayer.Framework.Model;
using NcmPlayer.Helpers;
using NcmPlayer.Views;
using NcmPlayer.Views.Pages;

namespace NcmPlayer.ViewModels;

public class MusicListDetailViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    private ImageBrush _cover;
    private string _createTime;
    private string _creator;
    private long _currentId;
    private string _description;
    private string _musicsCount;
    private string _name;
    private UIElementCollection _songViews;
    private MusicListDetailPage _musicListDetailPage;
    public Dictionary<long, Music> Musics = new();

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
            _musicsCount = $"{value} 首歌曲";
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
            _creator = $"By {value}";
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
        SetCover(new Uri(playList.PicUrl));
        var musics = await playList.InitArtWorkList();
        await UpdateMusicsList(musics, playList);
    }

    public async void PageLoaded(object sender, RoutedEventArgs e)
    {
        _musicListDetailPage = (MusicListDetailPage)sender;
        await LoadPlaylistAsync(_currentId);
    }

    public void SetCover(Uri picUrl)
    {
        BitmapImage image = new(picUrl);
        ImageBrush brush = new();
        brush.ImageSource = image;
        Cover = brush;
    }

    public async Task UpdateMusicsList(Music[] musics, PlayList list)
    {
        foreach (var music in musics)
        {
            Musics.Add(music.Id, music);
            await createAndUpdate(music);
        }
    }

    private async Task createAndUpdate(Music one)
    {
        var bitmap = new BitmapImage(new Uri(one.CoverUrl + "?param=40y40"));
        ImageBrush imageBrush = new()
        {
            ImageSource = bitmap
        };
        Border parent = new()
        {
            Tag = one.Id,
            BorderThickness = new Thickness(0),
            Margin = new Thickness(0, 0, 30, 0),
            CornerRadius = new CornerRadius(10, 10, 10, 10),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            Background = new SolidColorBrush(Color.FromArgb(1, 255, 255, 255))
        };
        Border bCover = new()
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(2),
            Width = 48,
            Height = 48,
            BorderThickness = new Thickness(0),
            CornerRadius = new CornerRadius(5),
            Background = imageBrush
        };
        Border corner = new()
        {
            CornerRadius = new CornerRadius(10, 10, 10, 10),
            Margin = new Thickness(5)
        };
        Grid content = new();
        var artists = one.ArtistsName;
        if (string.IsNullOrEmpty(one.ArtistsName))
        {
            artists = "未知艺人";
        }

        TextBlock textBlockName = new()
        {
            Text = one.Name,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Top,
            FontWeight = FontWeights.Black,
            Margin = new Thickness(60, 2, 0, 0),
            FontSize = 17,
            Width = 200,

            TextTrimming = TextTrimming.CharacterEllipsis
        };
        TextBlock textBlockArtists = new()
        {
            Text = artists,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
            Margin = new Thickness(60, 2, 0, 0),
            FontSize = 16,
            Width = 200,
            TextTrimming = TextTrimming.CharacterEllipsis
        };

        TextBlock textBlockTime = new()
        {
            Text = one.DuartionTimeString,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center,
            FontSize = 14,
            Margin = new Thickness(0, 0, 20, 0)
        };

        content.Children.Add(bCover);
        content.Children.Add(textBlockName);
        content.Children.Add(textBlockArtists);
        content.Children.Add(textBlockTime);
        corner.Child = content;
        parent.Child = corner;
        parent.PointerEntered += AnimationsHelper.CardShow;
        parent.PointerExited += AnimationsHelper.CardHide;
        parent.PointerPressed += PlayMusic;
        // 我认为这里应该是可以使用数据绑定的
        // qaq
        // TODO: 使用数据绑定歌单列表 SongViews.Add(parent);
        _musicListDetailPage.MusicsViewPanel.Children.Add(parent);
    }

    public void PlayMusic(object sender, PointerRoutedEventArgs e)
    {
        MusicPlayerHelper.Player.NewPlay(Musics[(long)((Border)sender).Tag]);
    }

}