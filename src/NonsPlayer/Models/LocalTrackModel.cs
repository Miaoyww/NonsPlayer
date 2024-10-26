using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using System.Text.Json.Serialization;

namespace NonsPlayer.Components.Models;

public class LocalTrackModel
{
    public string Title;
    public string Artist;
    public string Album;
    public string AlbumArtists;
    public string TrackNumber;
    public string Genre;
    public string Date;
}