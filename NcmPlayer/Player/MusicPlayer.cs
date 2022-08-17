using Microsoft.Win32;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Timers;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer.Player
{
    public static class MusicPlayer
    {
        public static MediaElement musicplayer = new();

        public static Timer updateInfo;
        public static Socket client_socket;

        public static void InitPlayer()
        {
            musicplayer.UnloadedBehavior = MediaState.Manual;
            musicplayer.LoadedBehavior = MediaState.Manual;
            musicplayer.MediaOpened += Musicplayer_MediaOpened;
            musicplayer.MediaFailed += Musicplayer_MediaFailed;
            musicplayer.MediaEnded += Musicplayer_MediaEnded;
            musicplayer.Visibility = Visibility.Hidden;
            musicplayer.Volume = Res.res.CVolume / 100;

            updateInfo = new();
            updateInfo.Elapsed += Timer_Elapsed;
            updateInfo.Interval = 100;
            updateInfo.Start();
        }

        // 信息更新
        public static void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            musicplayer.Dispatcher.BeginInvoke(new Action(() =>
            {
                Res.res.CPlayPostion = (int)musicplayer.Position.Duration().TotalSeconds;
                musicplayer.Volume = (double)Res.res.CVolume / 100;
            }));
        }

        public static void RePlay(string path, string name, string artists, Stream cover)
        {
            Res.res.CPlayName = name;
            Res.res.CPlayArtists = artists;
            Res.res.cSongPath = path;
            Res.res.Cover(cover);
            musicplayer.Source = new Uri(path);
            Play(true);
        }

        public static void Play(bool re = false)
        {
            if (!re)
            {
                if (!Res.res.IsPlaying)
                {
                    musicplayer.Play();
                    Res.res.IsPlaying = true;
                }
                else
                {
                    musicplayer.Pause();
                    Res.res.IsPlaying = false;
                }
            }
            else
            {
                Res.res.IsPlaying = true;
                musicplayer.Position = TimeSpan.Zero;
                musicplayer.Play();
            }
        }

        public static void Volume(double volume)
        {
            musicplayer.Volume = volume;
        }

        public static void Postion(int secs)
        {
            if (Res.res.IsPlaying)
            {
                musicplayer.Position = TimeSpan.FromSeconds(secs);
            }
        }

        public static void Musicplayer_MediaEnded(object sender, RoutedEventArgs e)
        {
            Res.res.IsPlaying = false;
        }

        public static void Musicplayer_MediaOpened(object sender, RoutedEventArgs e)
        {
            Res.res.CPlayWholeTime = (int)musicplayer.NaturalDuration.TimeSpan.TotalSeconds;
            Res.res.IsPlaying = true;
        }

        public static void Musicplayer_MediaFailed(object? sender, ExceptionRoutedEventArgs e)
        {
            throw new NotImplementedException();
        }

        /*
        public static void ReGetInfo(object source, ElapsedEventArgs e)
        {
            Thread thread = new(() =>
            {
                SendMessage("reget_info");
            });
            thread.Start();
            thread.IsBackground = true;
        }

        public static string B2S(byte[] bytes)
        {
            string msg = Encoding.UTF8.GetString(bytes);
            if (msg.Equals(""))
            {
                return string.Empty;
            }
            byte[] buffer = Encoding.UTF8.GetBytes(msg);
            string sResult = Encoding.UTF8.GetString(buffer, 0, buffer.Length);
            byte[] bomBuffer = new byte[] { 0xef, 0xbb, 0xbf };
            if (buffer[0] == bomBuffer[0] && buffer[1] == bomBuffer[1] && buffer[2] == bomBuffer[2])
            {
                int copyLength = buffer.Length - 3;
                byte[] dataNew = new byte[copyLength];
                Buffer.BlockCopy(buffer, 3, dataNew, 0, copyLength);
                sResult = Encoding.UTF8.GetString(dataNew);
            }
            return sResult;
        }

        public static void SendMessage(string cmd)
        {
            Thread send = new Thread(() =>
            {
                try
                {
                    client_socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
                    IPAddress ipAdress = IPAddress.Parse(Res.res.ServerIp);
                    //网络端点：为待请求连接的IP地址和端口号
                    IPEndPoint ipEndpoint = new IPEndPoint(ipAdress, int.Parse(Res.res.ServerPort));
                    while (true)
                    {
                        try
                        {
                            client_socket.Connect(ipEndpoint);
                            break;
                        }
                        catch
                        {
                            break;
                        }
                    }
                    if (client_socket.IsBound)
                    {
                        JObject jsonData = new JObject();
                        jsonData.Add("name", Environment.MachineName);
                        jsonData.Add("command", cmd);
                        client_socket.Send(Encoding.UTF8.GetBytes(jsonData.ToString()));
                        byte[] buffer = new byte[1024 * 1024 * 20];
                        int num = client_socket.Receive(buffer);
                        string reDataString = B2S(buffer);
                        if (reDataString != null)
                        {
                            try
                            {
                                JObject reData = (JObject)JsonConvert.DeserializeObject(reDataString);
                                if (reData != null)
                                {
                                    Pages.Player.playerPage.Dispatcher.BeginInvoke(new Action(() =>
                                    {
                                        Pages.Player.playerPage.tblock_title.Text = reData["title"].ToString();
                                        Pages.Player.playerPage.tblock_artists.Text = reData["artist"].ToString();
                                        byte[] imgBytes = Convert.FromBase64String(reData["albumPic"].ToString());
                                        imageStream = new MemoryStream(imgBytes);
                                        BitmapImage image = new BitmapImage();
                                        image.BeginInit();
                                        image.StreamSource = imageStream;
                                        image.EndInit();
                                        ImageBrush imgBrush = new ImageBrush();
                                        imgBrush.ImageSource = image;
                                        Pages.Player.playerPage.b_image.Background = imgBrush;
                                    }));
                                }

                                client_socket.Close();
                            }
                            catch (JsonReaderException)
                            {
                            }
                        }
                    }
                }
                catch (SocketException)
                {
                }
            });
            send.IsBackground = true;
            send.Start();
        }
        */
    }
}
