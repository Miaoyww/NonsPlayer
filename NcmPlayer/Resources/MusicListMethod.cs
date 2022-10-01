using NcmApi;
using NcmPlayer.Framework.Model;
using NcmPlayer.Views;
using NcmPlayer.Views.Pages;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer.Resources
{
    public class MusicVis
    {
        private int selfIndex;

        public int Index
        {
            set
            {
                selfIndex = value;
            }
            get
            {
                return selfIndex;
            }
        }

        public string Id { get; set; }
        public Music Music;
        public ListBoxItem View { get; set; }
    }

    public class MusicList
    {
        private int index;

        public int Index
        {
            set
            {
                index = value;
            }
            get
            {
                return index;
            }
        }

        public int Count { get => list.Count; }

        private List<MusicVis> list = new List<MusicVis>();

        public MusicVis Music2Vis(Music music)
        {
            MusicVis musicvis = new();
            musicvis.Music = music;
            ListBoxItem listBoxItem = new();
            Grid grid = new Grid();
            string artists = music.ArtistsName;
            TextBlock tblock_Name = new()
            {
                Text = music.Name,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14,
                Width = 130
            };
            TextBlock tblock_Artists = new()
            {
                Text = artists,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14
            };
            TextBlock tblock_Time = new()
            {
                Text = music.DuartionTimeString,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14
            };
            grid.Children.Add(tblock_Name);
            grid.Children.Add(tblock_Artists);
            grid.Children.Add(tblock_Time);
            listBoxItem.Content = grid;
            musicvis.Id = music.Id.ToString();
            musicvis.View = listBoxItem;
            return musicvis;
        }

        public void PostMusic(MusicVis music)
        {
            music.Index = list.Count - 1;
            if (IndexOf(music) == -1)
            {
                list.Add(music);
                ResEntry.res.CPlayCount = Count.ToString();
                UpdateIndex();
            }
        }

        private void PlayMusic(int index)
        {
            Thread playmusic = new(_ =>
            {
                if (list.Count != 0)
                {
                    Music music = list[index].Music;
                    string name = music.Name;
                    string artists = music.ArtistsName;
                    // Lrcs musicLrc = music.GetLrc;
                    MemoryStream ms = new();
                    Stream musicCover = music.GetPic().Result;
                    musicCover.CopyTo(ms);
                    string b64Cover = Convert.ToBase64String(ms.ToArray());
                    if (!string.IsNullOrEmpty(b64Cover))
                    {
                        Regediter.Regedit("Music", "MusicCover", b64Cover);
                    }
                    MemoryStream coverStream = new MemoryStream(Convert.FromBase64String(RegGeter.RegGet("Music", "MusicCover").ToString()));
                    string filepath = music.Url;
                    // string filepath = music.GetMp3();
                    MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
                    {
                        ResEntry.musicInfo.Cover(coverStream);
                        ResEntry.musicInfo.FilePath = filepath;
                        ResEntry.musicInfo.DurationTime = music.DuartionTime;
                        ResEntry.musicInfo.Id = music.Id.ToString();
                        ResEntry.musicInfo.AlbumCoverUrl = music.PicUrl;
                        ResEntry.musicInfo.AlbumId = music.Album.Id.ToString();
                        // ResEntry.musicInfo.LrcString = music.GetLrcString;
                        ResEntry.musicInfo.IsLiked = ResEntry.musicInfo.LikeList.Exists(t => t == music.Id);
                        MusicPlayer.RePlay(ResEntry.musicInfo.FilePath, name, artists);
                        Player.playerPage.ClearLrc();
                        // Player.playerPage.UpdateLrc(musicLrc);

                        Index = index;
                        UpdateIndex();
                    }));
                }
            });
            playmusic.IsBackground = true;
            playmusic.Start();
        }

        public void PlayByIndex(int index)
        {
            PlayMusic(index);
        }

        public void Play(MusicVis music)
        {
            int tempIndex = IndexOf(music);
            if (tempIndex != -1)
            {
                PlayMusic(tempIndex);
            }
        }

        public void Play(long id)
        {
            int tempIndex = IndexOf(id);
            if (tempIndex != -1)
            {
                PlayMusic(tempIndex);
            }
            else
            {
                PostMusic(Music2Vis(new Music(id)));
                PlayMusic(Count - 1);
            }
        }

        public void Last()
        {
            if (Count != 0)
            {
                int preCount = Index - 1;
                if (preCount >= 0)
                {
                    PlayByIndex(preCount);
                }
                else
                {
                    PlayByIndex(Count - 1);
                }
            }
        }

        public void Next()
        {
            if (Count != 0)
            {
                int preCount = Index + 1;
                if (preCount <= Count - 1)
                {
                    PlayByIndex(preCount);
                }
                else
                {
                    PlayByIndex(0);
                }
            }
        }

        public void Like()
        {
            if (Count != 0)
            {
                bool isliked = ResEntry.musicInfo.LikeList.Exists(t => t == int.Parse(ResEntry.musicInfo.Id));
                JObject result = Api.Music.Like(
                    ResEntry.musicInfo.Id,
                    !isliked,
                    ResEntry.ncm
                    ).Result;
                if ((int)result["code"] == 200)
                {
                    JArray likeListJson = (JArray)Api.User.Likelist(Login.acc.Id, ResEntry.ncm).Result["ids"];
                    string ids = string.Empty;
                    Login.acc.likelist.Clear();
                    foreach (int id in likeListJson)
                    {
                        Login.acc.likelist.Add(id);
                    }
                    Regediter.Regedit("Account", "Likelist", Convert.ToBase64String(Encoding.UTF8.GetBytes(ids)));
                    ResEntry.musicInfo.LikeList = Login.acc.likelist;
                    ResEntry.musicInfo.IsLiked = true;
                }
                else
                {
                    PublicMethod.SnackLog($"收藏失败: {result["msg"]}", "错误");
                }
            }
        }

        public void Clear()
        {
            list.Clear();
        }

        public void Add(MusicVis newone)
        {
            PostMusic(newone);
        }

        public void Remove(MusicVis removing)
        {
            list.Remove(removing);
            UpdateIndex();
        }

        public void Remove(int index)
        {
            list.RemoveAt(index);
            UpdateIndex();
        }

        public int IndexOf(MusicVis music)
        {
            for (int index = 0; index < list.Count; index++)
            {
                MusicVis item = list[index];
                if (item.Id == music.Id)
                {
                    return index;
                }
            }
            return -1;
        }

        public int IndexOf(long id)
        {
            for (int index = 0; index < list.Count; index++)
            {
                MusicVis item = list[index];
                if (item.Id == id.ToString())
                {
                    return index;
                }
            }
            return -1;
        }

        public MusicVis GetMusicVis(int index)
        {
            return list[index];
        }

        private void UpdateIndex()
        {
            int lastindex = -1;
            for (int index = 0; index < list.Count; index++)
            {
                if (list[index].Index - lastindex != 1)
                {
                    list[index].Index = index;
                }
                lastindex = index;
            }
        }
    }
}