using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Models;

public class PlayerState
{
    public TimeSpan Position
    {
        get;
        set;
    }
}