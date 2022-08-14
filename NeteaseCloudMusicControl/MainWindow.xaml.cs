using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Sockets;
using System.Net;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using System.Xml.Linq;
using Wpf.Ui.Controls;
using Wpf.Ui.Controls.Interfaces;
using Wpf.Ui.Mvvm.Contracts;
using Newtonsoft.Json.Linq;
using System.Threading;
using System.IO;
using Newtonsoft.Json;
using System.Timers;

namespace NeteaseCloudMusicControl
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : INavigationWindow
    {
        public System.Timers.Timer timer = new();
        public string ip = "127.0.0.1";
        public int port = 4000;
        public Socket client_socket;

        public MainWindow()
        {
            InitializeComponent();
            timer.Elapsed += new System.Timers.ElapsedEventHandler(ReGetInfo);
            timer.Interval = 1000;
            timer.Start();
        }

        public void ReGetInfo(object source, ElapsedEventArgs e)
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

        public void SendMessage(string cmd)
        {
            Thread send = new Thread(() =>
            {
                try
                {
                    client_socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
                    IPAddress ipAdress = IPAddress.Parse(ip);
                    //网络端点：为待请求连接的IP地址和端口号
                    IPEndPoint ipEndpoint = new IPEndPoint(ipAdress, port);
                    while (true)
                    {
                        try
                        {
                            client_socket.Connect(ipEndpoint);
                            break;
                        }
                        catch
                        {
                            Dispatcher.BeginInvoke(new Action(() =>
                            {
                                tblock_title.Text = "服务器连接失败, 请检查链接";
                                tblock_artists.Text = $"Ip: {ip}, Port: {port}";
                            }));
                            Thread.Sleep(1);
                        }
                    }
                    if (client_socket.IsBound)
                    {
                        JObject jsonData = new JObject();
                        jsonData.Add("name", Environment.MachineName);
                        jsonData.Add("command", cmd);
                        //发送消息到服务端
                        client_socket.Send(Encoding.UTF8.GetBytes(jsonData.ToString()));

                        byte[] buffer = new byte[2048 * 2048];
                        //接收服务端消息

                        int num = client_socket.Receive(buffer);
                        string reDataString = B2S(buffer);
                        if (reDataString != null)
                        {
                            JObject reData = (JObject)JsonConvert.DeserializeObject(reDataString);
                            if (reData != null)
                            {
                                Dispatcher.BeginInvoke(new Action(() =>
                                {
                                    tblock_title.Text = reData["title"].ToString();
                                    tblock_artists.Text = reData["artist"].ToString();
                                    byte[] imgBytes = Convert.FromBase64String(reData["albumPic"].ToString());
                                    Stream stream = new MemoryStream(imgBytes);
                                    BitmapImage image = new BitmapImage();
                                    image.BeginInit();
                                    image.StreamSource = stream;
                                    image.EndInit();
                                    ImageBrush imgBrush = new ImageBrush();
                                    imgBrush.ImageSource = image;
                                    b_image.Background = imgBrush;
                                }));
                            }
                        }
                        client_socket.Close();
                    }
                }
                catch (System.Net.Sockets.SocketException)
                {
                }
            });
            send.IsBackground = true;
            send.Start();
        }
        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            SendMessage("last");
        }

        private void btn_pause_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            SendMessage("pause");
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            SendMessage("next");
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            SendMessage("like");
        }

        private void Window_Loaded(object sender, RoutedEventArgs e)
        {
            Thread thread = new(() =>
            {
                SendMessage("connect");
            });
            thread.IsBackground = true;
            thread.Start();
        }

        #region Windows
        protected override void OnClosed(EventArgs e)
        {
            base.OnClosed(e);

            // Make sure that closing this window will begin the process of closing the application.
            Application.Current.Shutdown();
        }

        private void mitem_openWindow_Click(object sender, RoutedEventArgs e)
        {
            this.Show();
            this.Activate();
        }

        private void mitem_close_Click(object sender, RoutedEventArgs e)
        {
            this.Close();
        }

        private void Window_Closed(object sender, EventArgs e)
        {
            // 以防有资源未卸载的情况
            GC.Collect();
            GC.WaitForFullGCComplete();
            Environment.Exit(0);
        }

        #region INavigationWindow methods
        public Frame GetFrame()
        {
            throw new NotImplementedException();
        }

        public INavigation GetNavigation()
        {
            throw new NotImplementedException();
        }

        public bool Navigate(Type pageType)
        {
            throw new NotImplementedException();
        }

        public void SetPageService(IPageService pageService)
        {
            throw new NotImplementedException();
        }

        public void ShowWindow()
        {
            throw new NotImplementedException();
        }

        public void CloseWindow()
        {
            throw new NotImplementedException();
        }
        #endregion INavigationWindow methods

        #endregion Windows
    }
}
