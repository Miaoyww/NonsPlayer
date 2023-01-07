using System.Diagnostics;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NcmPlayer.Framework.Model;
using NcmPlayer.ViewModels;
using Windows.UI;

namespace NcmPlayer.Views;

public sealed partial class MusicListDetailPage : Page
{
    public MusicListDetailViewModel ViewModel
    {
        get;
    }

    public MusicListDetailPage()
    {
        ViewModel = App.GetService<MusicListDetailViewModel>();
        InitializeComponent();
    }

    private PlayList playList;
    private List<Music> InMusics;
    private int loadedViewCount = 0;
    public List<Music> musiclist = new List<Music>();

    public string Name
    {
        set => PlaylistName.Text = value;
    }

    public string CreateTime
    {
        set => PlaylistCreateTime.Text = $"· {value.Split(" ")[0]}";
    }

    public string MusicsCount
    {
        set => PlaylistMusicsCount.Text = $"{value} 首歌曲";
    }

    public string Description
    {
        set => PlaylistDescription.Text = value;
    }

    public string Creator
    {
        set => PlaylistCreator.Text = $"By {value}";
    }


    public void SetCover(Uri picUrl)
    {
        BitmapImage image = new(picUrl);
        ImageBrush brush = new();
        brush.ImageSource = image;
        PlaylistCover.Background = brush;
    }

    public async Task UpdateMusicsList(Music[] musics, PlayList list)
    {
        playList = list;
        InMusics = musics.ToList();
        if (playList.MusicsCount >= 100)
        {
            loadedViewCount = 100;
        }
        else
        {
            loadedViewCount = playList.MusicsCount;
        }
        updateList(0, loadedViewCount);
    }

    private async Task createAndUpdate(Music one, int index)
    {
        musiclist.Add(one);
        BitmapImage bitmap = new BitmapImage(new Uri(one.PicUrl + "?param=40y40"));
        ImageBrush imageBrush = new();
        imageBrush.ImageSource = bitmap;
        Border b_cover = new()
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(20, 0, 0, 0),
            Width = 40,
            Height = 40,
            CornerRadius = new CornerRadius(5),
            Background = imageBrush

        };
        Border parent = new()
        {
            Height = 50,
            Tag = one.Id,
            CornerRadius = new CornerRadius(10, 10, 10, 10),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            Background = new SolidColorBrush(Color.FromArgb(100, 212, 212, 212))
        };
        parent.Background.Opacity = 0;
        Border corner = new()
        {
            CornerRadius = new CornerRadius(10, 10, 10, 10),
            Margin = new Thickness(0, 5, 0, 5)
        };
        Grid content = new();
        string artists = one.ArtistsName;
        if (string.IsNullOrEmpty(one.ArtistsName))
        {
            artists = "未知艺人";
        }
        StackPanel musicInfo = new()
        {
            Margin = new Thickness(75, 0, 0, 0),
            Width = 400,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
        };
        TextBlock tblock_Name = new()
        {
            Text = one.Name,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            FontWeight = FontWeights.Bold,
            FontSize = 17,
            Width = 390,
            TextTrimming = TextTrimming.CharacterEllipsis
        };
        TextBlock tblock_Artists = new()
        {
            Text = artists,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            FontSize = 16,
            Width = 390,
            TextTrimming = TextTrimming.CharacterEllipsis
        };
        musicInfo.Children.Add(tblock_Name);
        musicInfo.Children.Add(tblock_Artists);
        TextBlock tblock_Time = new()
        {
            Text = one.DuartionTimeString,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center,
            FontSize = 14,
            Margin = new Thickness(0, 0, 20, 0)
        };

        content.Children.Add(b_cover);
        content.Children.Add(musicInfo);
        content.Children.Add(tblock_Time);
        corner.Child = content;
        parent.Child = corner;
        Musics.Children.Add(parent);
    }

    private async Task updateList(int start, int end)
    {
        Stopwatch sw = new();
        sw.Start();
        if (start != 0)
        {
            Musics.Children.RemoveAt(Musics.Children.Count - 1);
        }
        end = end - 1;
        for (int index = start; index <= end; index++)
        {
            await createAndUpdate(InMusics[index], index);
        }
    }
}