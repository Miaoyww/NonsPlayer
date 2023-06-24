using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework.Model;

namespace NonsPlayer.Components.Models
{
    public class PlaylistItem
    {
        public JObject PlayList
        {
            get;
            set;
        }
    }
}