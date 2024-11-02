﻿using Microsoft.Extensions.Logging;
using Microsoft.UI;
using Microsoft.UI.Input;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Vanara.PInvoke;
using Windows.Graphics;
using Windows.Win32;

namespace NonsPlayer.Helpers;

public static class WindowUtility
{
    private static readonly ILogger logger = App.GetLogger<App>();

    public static WindowId? CurrentWindowId;

    public static DisplayArea? CurrentWindowDisplayArea
    {
        get
        {
            if (!CurrentWindowId.HasValue)
            {
                return null;
            }

            return DisplayArea.GetFromWindowId(CurrentWindowId.Value, DisplayAreaFallback.Primary);
        }
    }

    internal static double CurrentWindowMonitorScaleFactor
        // Deliberate loss of precision
        // ReSharper disable once PossibleLossOfFraction
        => (CurrentWindowMonitorDpi * 100 + (96 >> 1)) / 96 / 100.0;

    public static nint CurrentWindowMonitorPtr
    {
        get
        {
            DisplayArea? displayArea = CurrentWindowDisplayArea;
            if (displayArea == null)
            {
                return nint.Zero;
            }

            nint monitorPtr = Win32Interop.GetMonitorFromDisplayId(displayArea.DisplayId);
            return monitorPtr;
        }
    }

    public static uint CurrentWindowMonitorDpi
    {
        get
        {
            const uint DefaultDpiValue = 96;
            nint monitorPtr = CurrentWindowMonitorPtr;
            if (monitorPtr == nint.Zero)
            {
                return DefaultDpiValue;
            }

            var dpi = User32.GetDpiForWindow(App.WindowHandle);
            if (dpi == 0)
            {
                return dpi;
            }

            logger.LogError(
                $"[WindowUtility::CurrentWindowMonitorDpi] Could not get DPI for the current monitor at 0x{monitorPtr:x8}");
            return DefaultDpiValue;
        }
    }

    public static double UiScale => User32.GetDpiForWindow(new HWND(App.WindowHandle)) / 96d;

    public delegate int SetTitleBarDragRegionDelegate(InputNonClientPointerSource source, SizeInt32 size,
        double scaleFactor, Func<UIElement, RectInt32?, RectInt32> getScaledRect);

    /// <summary>
    /// Informs the bearer to refresh the drag region.
    /// will not set<see cref="NonClientRegionKind.LeftBorder"/>, <see cref="NonClientRegionKind.RightBorder"/>, <see cref="NonClientRegionKind.Caption"/>when titleBarHeight less than 0
    /// </summary>
    /// <param name="element"></param>
    /// <param name="window"></param>
    /// <param name="setTitleBarDragRegion"></param>
    public static void RaiseSetTitleBarDragRegion(this Window window,
        SetTitleBarDragRegionDelegate setTitleBarDragRegion)
    {
        if (!window.AppWindow.IsVisible)
            return;
        // UIElement.RasterizationScale is always 1
        var source = InputNonClientPointerSource.GetForWindowId(window.AppWindow.Id);
        var uiElement = window.Content;
        var xamlRoot = uiElement?.XamlRoot;

        if (xamlRoot is null)
            return;

        var scaleFactor = xamlRoot.RasterizationScale;
        var size = window.AppWindow.Size;
        // If the number of regions is 0 or 1, AppWindow will automatically reset to the default region next time, but if it is >=2, it will not and need to be manually cleared
        source.ClearRegionRects(NonClientRegionKind.Passthrough);
        var titleBarHeight = setTitleBarDragRegion(source, size, scaleFactor, GetScaledRect);
        if (titleBarHeight >= 0)
        {
            // region under the buttons
            const int borderThickness = 5;
            source.SetRegionRects(NonClientRegionKind.LeftBorder,
                [GetScaledRect(uiElement, new(0, 0, borderThickness, titleBarHeight))]);
            source.SetRegionRects(NonClientRegionKind.RightBorder,
                [GetScaledRect(uiElement, new(size.Width, 0, borderThickness, titleBarHeight))]);
            source.SetRegionRects(NonClientRegionKind.Caption,
                [GetScaledRect(uiElement, new(0, 0, size.Width, titleBarHeight))]);
        }
    }

    private static RectInt32 GetScaledRect(this UIElement uiElement, RectInt32? r = null)
    {
        if (r is { } rect)
        {
            var scaleFactor = uiElement.XamlRoot.RasterizationScale;
            return new((int)(rect.X * scaleFactor), (int)(rect.Y * scaleFactor), (int)(rect.Width * scaleFactor),
                (int)(rect.Height * scaleFactor));
        }
        else
        {
            var pos = uiElement.TransformToVisual(null).TransformPoint(new(0, 0));
            rect = new RectInt32((int)pos.X, (int)pos.Y, (int)uiElement.ActualSize.X, (int)uiElement.ActualSize.Y);
            return GetScaledRect(uiElement, rect);
        }
    }
}