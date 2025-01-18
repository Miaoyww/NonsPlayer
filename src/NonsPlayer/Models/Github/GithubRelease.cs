using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Models.Github;

public class GithubRelease
{
    public GithubAsset[] Assets;
    public DateTime UpdateTime;
    public GithubRelease(JObject content)
    {
        if (content.ContainsKey("message"))
        {
            ExceptionService.Instance.Throw(content.GetValue("message").ToString());
            return;
        }
        var resource = (JArray)content.GetValue("assets");
        Assets = new GithubAsset[resource.Count];
        for (int i = 0; i < Assets.Length; i++)
        {
            Assets[i] = JsonConvert.DeserializeObject<GithubAsset>(resource[i].ToString());
        }

        UpdateTime = DateTime.Parse(content.GetValue("published_at").ToString());
    }
}