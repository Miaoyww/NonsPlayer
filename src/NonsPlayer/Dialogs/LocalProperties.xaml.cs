using ATL;
using System.Windows.Forms;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using NonsPlayer.Updater.Update;
using System.Text.RegularExpressions;
using System.Diagnostics;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Dialogs;

[INotifyPropertyChanged]
public sealed partial class LocalProperties : Page
{
    private LocalMusic currentMusic;
    [ObservableProperty] private string title;
    [ObservableProperty] private string artist;
    [ObservableProperty] private string album;
    [ObservableProperty] private string albumArtists;
    [ObservableProperty] private string trackNumber;
    [ObservableProperty] private string genre;
    [ObservableProperty] private string date;
    [ObservableProperty] private string duration;
    [ObservableProperty] private string bitRate;
    [ObservableProperty] private string codec;
    [ObservableProperty] private string fileSize;
    [ObservableProperty] private string filePath;

    private LocalTrackModel Model = new();
    public LocalProperties(IMusic music)
    {
        InitializeComponent();
        if (music is LocalMusic)
        {
            currentMusic = music as LocalMusic;
            Init(currentMusic.FilePath);
        }

        this.Tag = Model;
        TitleTextBlock.Text = "Title".GetLocalized();
        ArtistTextBlock.Text = "Artist".GetLocalized();
        AlbumTextBlock.Text = "Album".GetLocalized();
        AlbumArtistsTextBlock.Text = "AlbumArtists".GetLocalized();
        TrackNumberTextBlock.Text = "TrackNumber".GetLocalized();
        GenreTextBlock.Text = "Genre".GetLocalized();
        DateTextBlock.Text = "Date".GetLocalized();
        DurationTextBlock.Text = "Duration".GetLocalized();
        BitRateTextBlock.Text = "BitRate".GetLocalized();
        CodecTextBlock.Text = "Codec".GetLocalized();
        FileSizeTextBlock.Text = "FileSize".GetLocalized();
        FilePathTextBlock.Text = "FilePath".GetLocalized();
    }

    public void Init(string path)
    {
        var track = new Track(path);
        var fileInfo = new FileInfo(path);
        Title = track.Title;
        Artist = track.Artist;
        Album = track.Album;
        AlbumArtists = track.AlbumArtist;
        TrackNumber = track.TrackNumber.ToString();
        Genre = track.Genre;
        Date = track.Year.ToString();
        Duration = TimeSpan.FromMilliseconds(track.DurationMs).ToString();
        BitRate = track.Bitrate + " kbps";
        Codec = fileInfo.Extension;
        double mb = 1 << 20;
        FileSize = $"{fileInfo.Length / mb:F2} MB";
        FilePath = path;
    }

    [RelayCommand]
    public void OpenFilePath()
    {
        Process.Start("explorer.exe", $"/select,\"{FilePath}\"");
    }

    partial void OnTitleChanged(string value)
    {
        Model.Title = value;
    }

    partial void OnArtistChanged(string value)
    {
        Model.Artist = value;
    }

    partial void OnAlbumChanged(string value)
    {
        Model.Album = value;
    }

    partial void OnAlbumArtistsChanged(string value)
    {
        Model.AlbumArtists = value;
    }

    partial void OnTrackNumberChanged(string value)
    {
        Model.TrackNumber = value;
    }

    partial void OnGenreChanged(string value)
    {
        Model.Genre = value;
    }

    partial void OnDateChanged(string value)
    {
        Model.Date = value;
    }

    private void OnPreviewKeyDown(object sender, KeyRoutedEventArgs e)
    {
        e.Handled = !IsTextAllowed(((NumberBox)sender).Text);
    }

    private static bool IsTextAllowed(string text)
    {
        var regex = new Regex("^[1-9][0-9]*$");
        return regex.IsMatch(text);
    }
}