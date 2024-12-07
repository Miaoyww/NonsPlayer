namespace NonsPlayer.DataBase.Models;


public class DbSongModel
{
    public int Id { get; set; }
    public string Path { get; set; }
    public string Title { get; set; }
    public int Track { get; set; }
    public string Album { get; set; }
    public string Artist { get; set; }
    public int Duration { get; set; }

    public int Bitrate { get; set; }
    public long Created { get; set; }
    public int? Rate { get; set; }
}