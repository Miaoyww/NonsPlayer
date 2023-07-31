using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Helpers;
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
            // var savedPlaylistTasks = new List<Task<Playlist>>();
            var userPlaylistTasks = new List<Task<Playlist>>();
            foreach (var playlistItem in result)
            {
                if (playlistItem["name"].ToString() == Account.Instance.Name + "喜欢的音乐")
                {
                    var likedSongs =
                        (JArray)(await Apis.Playlist.Detail(
                                long.Parse(playlistItem["id"].ToString()), Nons.Instance).ConfigureAwait(false)
                        )["playlist"]["trackIds"];
                    LikedSongs = likedSongs.Select(likedSong => likedSong["id"].ToString()).ToArray();
                }

                var playlistTask = CacheHelper.GetPlaylistCardAsync(
                    playlistItem["id"] + "_playlist", new JObject
                    {
                        new JProperty("id", playlistItem["id"]),
                        new JProperty("name", playlistItem["name"]),
                        new JProperty("picUrl", playlistItem["coverImgUrl"]),
                    });
                if ((bool)playlistItem["subscribed"])
                {
                    // savedPlaylistTasks.Add(playlistTask);
                }
                else
                {
                    userPlaylistTasks.Add(playlistTask);
                }
            }

            var whenAllResult = await Task
                .WhenAll(userPlaylistTasks)
                .ConfigureAwait(false);
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                // whenAllResult.ToList().ForEach(item => SavedPlaylists.Add(item));
                whenAllResult.ToList().ForEach(item => UserPlaylists.Add(item));
            });
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}