using Microsoft.Win32;
using NcmApi;
using NcmPlayer.CloudMusic;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Markup;
using System.Windows.Media.Imaging;

namespace NcmPlayer.Views.Pages.LoginPage
{
    public partial class UnLoggedin : Page
    {
        public UnLoggedin()
        {
            InitializeComponent();
            DataContext = ResEntry.res;

        }

        private void btn_login_Click(object sender, RoutedEventArgs e)
        {
            if (!ResEntry.ncm.isLoggedin)
            {
                if (!string.IsNullOrEmpty(tbox_account.Text) || string.IsNullOrEmpty(tbox_password.Text))
                {
                    string email = tbox_account.Text;
                    int phone;
                    if (int.TryParse(tbox_account.Text, out phone))
                    {
                        try
                        {
                            ResEntry.ncm.Login(phone, tbox_password.Text);
                            Login.acc.Encrypt(ResEntry.ncm.Token);
                            Login.acc.CheckLogin();
                        }
                        catch (NcmApi.LoginFailed error)
                        {
                            PublicMethod.ShowDialog(error.ToString(), "错误");
                        }
                    }
                    else
                    {
                        try
                        {
                            ResEntry.ncm.Login(email, tbox_password.Password);
                            Login.acc.Encrypt(ResEntry.ncm.Token);
                            Login.acc.CheckLogin();
                        }
                        catch (NcmApi.LoginFailed error)
                        {
                            PublicMethod.ShowDialog(error.ToString(), "错误");
                        }
                    }
                }
                else
                {
                    PublicMethod.ShowDialog("请输入账号密码!");
                }

            }
            else
            {
                Login.acc.CheckLogin();
            }
        }
    }
}