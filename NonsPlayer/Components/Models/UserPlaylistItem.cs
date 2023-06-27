using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml.Media;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework.Model;

namespace NonsPlayer.Components.Models
{
    public class UserPlaylistItem
    {
        public JObject PlayList
        {
            get;
            set;
        }
    }
}