using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Contracts.Models.Nons;

public interface IAccount : INonsModel
{
    string NickName { get; set; }
    string Token { get; set; }
    string Uid { get; set; }
    string FaceUrl { get; set; }
}