using System.Collections.ObjectModel;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Framework.Model;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.CommonPages;

public sealed partial class MusicListDetailPage : Page
{
    public MusicListDetailViewModel ViewModel
    {
        get;
    }
    public class Musics : ObservableCollection<Music>
    {
        public Musics() : base()
        {
        }
    }

    public StackPanel MusicsViewPanel;
    public MusicListDetailPage()
    {
        ViewModel = App.GetService<MusicListDetailViewModel>();
        InitializeComponent();
        // MusicsViewPanel = MusicsPanel;
    }

}