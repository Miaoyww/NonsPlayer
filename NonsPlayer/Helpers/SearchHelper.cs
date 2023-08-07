using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Helpers
{
    [INotifyPropertyChanged]
    public partial class SearchHelper
    {
        public static SearchHelper Instance { get; set; } = new();
        [ObservableProperty] private Music bestMusicResult;

        public delegate void BestMusicResultChangedHandler(Music value);

        public event BestMusicResultChangedHandler BestMusicResultChanged;

        partial void OnBestMusicResultChanged(Music value)
        {
            BestMusicResultChanged(value);
        }
    }

    public enum SearchDataType
    {
        Unknown,
        Music,
        Artist,
        Playlist,
        Album
    }
}