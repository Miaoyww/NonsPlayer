using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsApi;
using NonsPlayer.Components.Models;
using NonsPlayer.Services;

namespace NonsPlayer.Models
{
    public class UserPlaylistMethods
    {
        public static UserPlaylistMethods Instance
        {
            get;
        } = new();

        public ObservableCollection<UserPlaylistItem> UserPlaylists
        {
            get;
        } = new();

        public ObservableCollection<UserPlaylistItem> FavoritePlaylists
        {
            get;
        } = new();

        public async void Init()
        {
            var result = (JArray)(await Api.User.Playlist(AccountService.Instance.Uid, Nons.Instance))["playlist"];
            foreach (var playlistItem in result)
            {
                if (playlistItem["creator"]["userId"].ToString() == AccountService.Instance.Uid)
                {
                    UserPlaylists.Add(new UserPlaylistItem
                    {
                        PlayList = (JObject)playlistItem
                    });
                }
                else
                {
                    FavoritePlaylists.Add(new UserPlaylistItem
                    {
                        PlayList = (JObject)playlistItem
                    });
                }
            }
        }
    }
}