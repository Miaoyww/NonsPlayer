using NcmPlayer.CloudMusic;
using NcmPlayer.Views;
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer.Resources
{
    public class SongVis
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
        public Song Song;
        public ListBoxItem View { get; set; }
    }

    public class PlayList
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

        private List<SongVis> list = new List<SongVis>();

        public SongVis Song2Vis(Song song)
        {
            SongVis songvis = new();
            songvis.Song = song;
            ListBoxItem listBoxItem = new();
            Grid grid = new Grid();
            string artists = string.Empty;
            for (int i = 0; i <= song.Artists.Length - 1; i++)
            {
                if (i != song.Artists.Length - 1)
                {
                    artists += song.Artists[i] + "/";
                }
                else
                {
                    artists += song.Artists[i];
                }
            }
            TextBlock tblock_Name = new()
            {
                Text = song.Name,
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
                Text = song.DuartionTimeString,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14
            };
            grid.Children.Add(tblock_Name);
            grid.Children.Add(tblock_Artists);
            grid.Children.Add(tblock_Time);
            listBoxItem.Content = grid;
            songvis.Id = song.Id;
            songvis.View = listBoxItem;
            return songvis;
        }

        public void PostSong(SongVis song)
        {
            song.Index = list.Count - 1;
            if (IndexOf(song) == -1)
            {
                list.Add(song);
                ResEntry.res.CPlayCount = Count.ToString();
                UpdateIndex();
            }
        }

        private void PlaySong(int index)
        {
            ThreadPool.QueueUserWorkItem(_ =>
            {
                MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
                {
                    if (list.Count != 0)
                    {
                        Song song = list[index].Song;
                        string[] artist = song.Artists;
                        string name = song.Name;
                        string artists = string.Empty;
                        Lrcs songLrc = song.GetLrc;
                        for (int i = 0; i <= artist.Length - 1; i++)
                        {
                            if (i != artist.Length - 1)
                            {
                                artists += artist[i] + "/";
                            }
                            else
                            {
                                artists += artist[i];
                            }
                        }
                        MemoryStream ms = new();
                        Stream songCover = song.Cover;
                        songCover.CopyTo(ms);
                        string b64Cover = Convert.ToBase64String(ms.ToArray());
                        if (!string.IsNullOrEmpty(b64Cover))
                        {
                            Regediter.Regedit("Song", "SongCover", b64Cover);
                        }
                        ResEntry.songInfo.Cover(new MemoryStream(Convert.FromBase64String(RegGeter.RegGet("Song", "SongCover").ToString())));
                        ResEntry.songInfo.FilePath = song.GetMp3();
                        MusicPlayer.RePlay(ResEntry.songInfo.FilePath, name, artists);
                        ResEntry.songInfo.DurationTime = song.DuartionTime;
                        ResEntry.songInfo.AlbumCoverUrl = song.CoverUrl;
                        ResEntry.songInfo.AlbumId = song.AlbumId;
                        ResEntry.songInfo.LrcString = song.GetLrcString;

                        Views.Pages.Player.playerPage.ClearLrc();
                        Views.Pages.Player.playerPage.UpdateLrc(songLrc);

                        Index = index;
                        UpdateIndex();
                    }
                }));
            });
        }

        public void Play(int index)
        {
            PlaySong(index);
        }

        public void Play(SongVis song)
        {
            int tempIndex = IndexOf(song);
            if (tempIndex != -1)
            {
                PlaySong(tempIndex);
            }
        }

        public void Play(string id)
        {
            int tempIndex = IndexOf(id);
            if (tempIndex != -1)
            {
                PlaySong(tempIndex);
            }
            else
            {
                PostSong(Song2Vis(new Song(id)));
                PlaySong(Count - 1);
            }
        }

        public void Last()
        {
            if (Count != 0)
            {
                int preCount = Index - 1;
                if (preCount >= 0)
                {
                    Play(preCount);
                }
                else
                {
                    Play(Count - 1);
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
                    Play(preCount);
                }
                else
                {
                    Play(0);
                }
            }
        }

        public void Clear()
        {
            list.Clear();
        }

        public void Add(SongVis newone)
        {
            PostSong(newone);
        }

        public void Remove(SongVis removing)
        {
            list.Remove(removing);
            UpdateIndex();
        }

        public void Remove(int index)
        {
            list.RemoveAt(index);
            UpdateIndex();
        }

        public int IndexOf(SongVis song)
        {
            for (int index = 0; index < list.Count; index++)
            {
                SongVis item = list[index];
                if (item.Id == song.Id)
                {
                    return index;
                }
            }
            return -1;
        }

        public int IndexOf(string id)
        {
            for (int index = 0; index < list.Count; index++)
            {
                SongVis item = list[index];
                if (item.Id == id)
                {
                    return index;
                }
            }
            return -1;
        }

        public SongVis GetSongVis(int index)
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