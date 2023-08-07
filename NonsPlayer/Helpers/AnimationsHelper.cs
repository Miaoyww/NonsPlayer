using System.Numerics;
using Microsoft.UI.Composition;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;

namespace NonsPlayer.Helpers;

public static class AnimationsHelper
{
    private static readonly Compositor _compositor = CompositionTarget.GetCompositorForCurrentThread();
    private static SpringVector3NaturalMotionAnimation _springAnimation;

    private static void UpdateSpringAnimation(float finalValue)
    {
        if (_springAnimation == null)
        {
            _springAnimation = _compositor.CreateSpringVector3Animation();
            _springAnimation.Target = "Scale";
        }

        _springAnimation.FinalValue = new Vector3(finalValue);
        _springAnimation.DampingRatio = 1f;
        _springAnimation.Period = new TimeSpan(350000);
    }

    public static void CardHide(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1f);

        StartAnimationIfAPIPresent(sender as UIElement, _springAnimation);
    }

    public static void CardShow(object sender, PointerRoutedEventArgs e)
    {
        UpdateSpringAnimation(1.038f);

        StartAnimationIfAPIPresent(sender as UIElement, _springAnimation);
    }

    private static void StartAnimationIfAPIPresent(UIElement sender, CompositionAnimation animation)
    {
        sender.StartAnimation(animation);
    }
}