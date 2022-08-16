using Microsoft.Win32;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Timers;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Wpf.Ui.Controls.Interfaces;
using Wpf.Ui.Mvvm.Contracts;

namespace NeteaseCloudMusicControl.Views.Methods
{
    public static class PlayerMethods
    {
        public static System.Timers.Timer timer;
        public static Socket client_socket;
        public static Stream imageStream;


        public static void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            CurrentResources.musicplayer.Dispatcher.BeginInvoke(new Action(() =>
            {
                if (!CurrentResources.isPlayed)
                {
                    CurrentResources.musicplayer.Volume += 0.1;
                    if (CurrentResources.musicplayer.Volume - (double.Parse(CurrentResources.currentVolume) / 100) <= 0.005)
                    {
                        timer.Stop();
                        CurrentResources.isPlayed = true;
                    }
                }
                else
                {
                    CurrentResources.musicplayer.Volume -= 0.1;
                    if(CurrentResources.musicplayer.Volume <= 0)
                    {
                        CurrentResources.musicplayer.Pause();
                        timer.Stop();
                        CurrentResources.isPlayed = false;
                    }
                }

            }));
        }

        public static void RePlay(string path)
        {
            CurrentResources.soundPath = path;
            CurrentResources.musicplayer.Source = new Uri(path);
            Play(true);
        }

        public static void Play(bool re = false)
        {
            if (!re)
            {
                if (!CurrentResources.isPlayed)
                {
                    CurrentResources.musicplayer.Play();
                    CurrentResources.isPlayed = true;
                }
                else
                {
                    CurrentResources.musicplayer.Pause();
                    CurrentResources.isPlayed = false;
                }
            }
            else
            {
                CurrentResources.isPlayed = true;
                CurrentResources.musicplayer.Position = TimeSpan.Zero;
                CurrentResources.musicplayer.Play();
            }
        }

        public static void Volume(double volume)
        {
            CurrentResources.musicplayer.Volume = volume;
        }

        public static void Musicplayer_MediaEnded(object sender, RoutedEventArgs e)
        {
            CurrentResources.isPlayed = false;
        }

        public static void Musicplayer_MediaOpened(object sender, RoutedEventArgs e)
        {
            CurrentResources.currentPlayWholeTime = (int)CurrentResources.musicplayer.NaturalDuration.TimeSpan.TotalSeconds;
            CurrentResources.isPlayed = true;
        }

        public static void Musicplayer_MediaFailed(object? sender, ExceptionRoutedEventArgs e)
        {
            throw new System.NotImplementedException();
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
                    IPAddress ipAdress = IPAddress.Parse(CurrentResources.ServerIp);
                    //网络端点：为待请求连接的IP地址和端口号
                    IPEndPoint ipEndpoint = new IPEndPoint(ipAdress, int.Parse(CurrentResources.ServerPort));
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
