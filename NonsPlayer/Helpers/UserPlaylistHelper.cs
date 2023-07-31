using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models;

namespace NonsPlayer.Heplers
{
    public class UserPlaylistHelper : INotifyPropertyChanged
    {
        private bool _currentSongLiked = false;

        public static UserPlaylistHelper Instance
        {
            get;
        } = new();

        public ObservableCollection<Playlist> UserPlaylists
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
        public bool IsLiked(long? id)
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

        private void OnMusicChanged(Music music)
        {
            CurrentSongLiked = IsLiked(music.Id);
        }

        public UserPlaylistHelper()
        {
            Player.Instance.MusicChangedHandle += OnMusicChanged;
        }

        public ObservableCollection<Playlist> SavedPlaylists
        {
            get;
        } = new();

        public async void Init()
        {
            var result = (JArray)(await Apis.User.Playlist(Account.Instance.Uid, Nons.Instance))["playlist"];
            List<Task<Playlist>> savedPlaylistTasks = new();
            List<Task<Playlist>> userPlaylistTasks = new();
            foreach (var playlistItem in result)
            {
                if (playlistItem["name"].ToString() == Account.Instance.Name + "喜欢的音乐")
                {
                    var likedSongs =
                        (JArray)(await Apis.Playlist.Detail(
                            long.Parse(playlistItem["id"].ToString()), Nons.Instance)
                        )["playlist"]["trackIds"];
                    LikedSongs = likedSongs.Select(likedSong => likedSong["id"].ToString()).ToArray();
                }

                if ((bool)playlistItem["subscribed"])
                {
                    savedPlaylistTasks.Add(Playlist.CreateAsync(long.Parse(playlistItem["id"].ToString())));
                }
                else
                {
                    userPlaylistTasks.Add(Playlist.CreateAsync(long.Parse(playlistItem["id"].ToString())));
                }
            }

            var whenAllResult = await Task.WhenAll(Task.WhenAll(savedPlaylistTasks), Task.WhenAll(userPlaylistTasks));
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                whenAllResult[0].ToList().ForEach(item => SavedPlaylists.Add(item));
                whenAllResult[1].ToList().ForEach(item => UserPlaylists.Add(item));
            });
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}