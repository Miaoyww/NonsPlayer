using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Dispatching;
using NonsPlayer.Contracts.Services;

namespace NonsPlayer.Framework.Resources
{
    public static class ServiceEntry
    {
        public static DispatcherQueue DispatcherQueue
        {
            get;
            set;
        }
        public static INavigationService NavigationService
        {
            get;
            set;
        }
    }
}