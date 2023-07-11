using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Framework;
using NonsPlayer.Framework.Api;
using NonsPlayer.Framework.Model;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Services;

namespace NonsPlayer.Heplers
{
    public class UserPlaylistHelper : INotifyPropertyChanged
    {
        private bool _currentSongLiked = false;

        public static UserPlaylistHelper Instance
        {
            get;
        } = new();

        public ObservableCollection<UserPlaylistItem> UserPlaylists
        {
            get;
        } = new();

        public string[] LikedSongs
        {
            get;
            set;
        }

        public bool CurrentSongLiked
        {
            get => _currentSongLiked;
            set
            {
                _currentSongLiked = value;
                OnPropertyChanged();
            }
        }

        // 用于判断当前音乐是否已收藏
        public bool IsLiked(long id)
        {
            if (LikedSongs.Contains(id.ToString()))
            {
                return true;
            }
            else
            {
                return false;
            }
        }
        private Music OnMusicChanged(Music music)
        {
            CurrentSongLiked = IsLiked(music.Id);
            return music;
        }

        public UserPlaylistHelper()
        {
            GlobalMusicState.Instance.MusicChangedHandle += OnMusicChanged;
        }

        public ObservableCollection<UserPlaylistItem> SavedPlaylists
        {
            get;
        } = new();

        public async void Init()
        {
            var a = await Apis.User.Playlist(AccountService.Instance.Uid, Nons.Instance);
            var result = (JArray)(await Apis.User.Playlist(AccountService.Instance.Uid, Nons.Instance))["playlist"];
            foreach (var playlistItem in result)
            {
                if (playlistItem["name"].ToString() == AccountService.Instance.Name + "喜欢的音乐")
                {
                    var likedSongs =
                        (JArray)(await Apis.Playlist.Detail(
                            long.Parse(playlistItem["id"].ToString()), Nons.Instance)
                        )["playlist"]["trackIds"];
                    LikedSongs = likedSongs.Select(likedSong => likedSong["id"].ToString()).ToArray();
                }

                if ((bool)playlistItem["subscribed"])
                {
                    SavedPlaylists.Add(new UserPlaylistItem
                    {
                        PlayList = (JObject)playlistItem
                    });
                }
                else
                {
                    UserPlaylists.Add(new UserPlaylistItem
                    {
                        PlayList = (JObject)playlistItem
                    });
                }
            }
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}