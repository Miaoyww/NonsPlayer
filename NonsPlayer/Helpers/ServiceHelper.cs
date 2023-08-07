﻿using Microsoft.UI.Dispatching;
using NonsPlayer.Contracts.Services;

namespace NonsPlayer.Helpers;

public static class ServiceHelper
{
    public static DispatcherQueue DispatcherQueue { get; set; }

    public static INavigationService NavigationService { get; set; }
}