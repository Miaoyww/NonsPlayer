using System.Numerics;
using Microsoft.UI.Composition;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
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

    #region ui动画，我求你了，别来折磨我了我求你了

    private Compositor _compositor = CompositionTarget.GetCompositorForCurrentThread();
    private SpringVector3NaturalMotionAnimation _springAnimation;

    private void UpdateSpringAnimation(float finalValue)
    {
        if (_springAnimation == null)
        {
            _springAnimation = _compositor.CreateSpringVector3Animation();
            _springAnimation.Target = "Scale";
        }

        _springAnimation.FinalValue = new Vector3(finalValue);
        _springAnimation.DampingRatio = 1f;
        _springAnimation.Period = new TimeSpan(350000);
    }

    public void CardHide(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1f);

        StartAnimationIfAPIPresent((sender as UIElement), _springAnimation);
    }

    public void CardShow(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1.038f);

        StartAnimationIfAPIPresent((sender as UIElement), _springAnimation);
    }

    #endregion ui动画，我求你了，别来折磨我了我求你了

    private void StartAnimationIfAPIPresent(UIElement sender, Microsoft.UI.Composition.CompositionAnimation animation)
    {
        (sender as UIElement).StartAnimation(animation);
    }

    private PlayList playList;
    private int loadedViewCount = 0;
    public List<Music> Musics = new List<Music>();

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
        if (playList.MusicsCount >= 100)
        {
            loadedViewCount = 100;
        }
        else
        {
            loadedViewCount = playList.MusicsCount;
        }
        foreach (var music in musics)
        {
            Musics.Add(music);
            await createAndUpdate(music);
        }
    }

    private async Task createAndUpdate(Music one)
    {
        Musics.Add(one);
        BitmapImage bitmap = new BitmapImage(new Uri(one.PicUrl + "?param=40y40"));
        ImageBrush imageBrush = new();
        imageBrush.ImageSource = bitmap;
        Border parent = new()
        {
            Tag = one.Id,
            BorderThickness = new Thickness(0),
            Margin = new Thickness(0, 0, 30, 0),
            CornerRadius = new CornerRadius(10, 10, 10, 10),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            Background = new SolidColorBrush(Color.FromArgb(1, 255, 255, 255))
        };
        Border b_cover = new()
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
        TextBlock tblock_Name = new()
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
        TextBlock tblock_Artists = new()
        {
            Text = artists,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
            Margin = new Thickness(60, 2, 0, 0),
            FontSize = 16,
            Width = 200,
            TextTrimming = TextTrimming.CharacterEllipsis
        };

        TextBlock tblock_Time = new()
        {
            Text = one.DuartionTimeString,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center,
            FontSize = 14,
            Margin = new Thickness(0, 0, 20, 0)
        };

        content.Children.Add(b_cover);
        content.Children.Add(tblock_Name);
        content.Children.Add(tblock_Artists);
        content.Children.Add(tblock_Time);
        corner.Child = content;
        parent.Child = corner;
        parent.PointerEntered += CardShow;
        parent.PointerExited += CardHide;
        PanelMusics.Children.Add(parent);
    }
}