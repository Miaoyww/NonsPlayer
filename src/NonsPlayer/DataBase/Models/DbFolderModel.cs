namespace NonsPlayer.DataBase.Models;

public class DbFolderModel
{
    public int Id { get; set; }
    public string Path { get; set; }
    public int LastModified { get; set; }
    public List<DbSongModel> Songs { get; set; }
}