using Newtonsoft.Json.Linq;
using NonsPlayer.Helpers;
using RestSharp;

namespace NonsPlayer.Models.Github;

public class PluginModel
{
    public string? Name { get; set; }
    public string? Slug { get; set; }
    public Version? Version { get; set; }
    public string? Author { get; set; }
    public Uri? AuthorLink { get; set; }
    public string? Description { get; set; }
    public string? Preview { get; set; }
    public int? Stars { get; set; }
    public string? Type { get; set; }
    public DateTime? UpdateTime { get; set; }
    public List<string>? Requirements { get; set; }
    public PluginModel Model { get; set; }
    public Uri? RepoUrl { get; set; }
    public string? RepoName { get; set; }
    public string? RepoBranch { get; set; }
    public string PluginInfoFile { get; set; }
    public GithubRelease? Release { get; set; }

    public bool IsInitialized { get; set; }

    public PluginModel(string url)
    {
        Model = this;
        PluginInfoFile = url;
    }

    public async Task GetPluginInfoAsync(RestRequest request)
    {
        var response = await new RestClient(PluginInfoFile).ExecuteAsync(request);
        var pluginInfo = JObject.Parse(response.Content);
        Name = pluginInfo.GetValue("name").ToString();
        Author = pluginInfo.GetValue("author").ToString();
        RepoName = pluginInfo.GetValue("repo").ToString();
        RepoBranch = pluginInfo.GetValue("branch").ToString();
        RepoUrl = new Uri($"https://github.com/{RepoName}");
        Release = new GithubRelease(JObject.Parse(
            (await new RestClient($"https://api.github.com/repos/{RepoName}/releases/latest")
                .ExecuteAsync(request)).Content));

        UpdateTime = Release.UpdateTime;
        Type = pluginInfo.GetValue("type")?.ToString() ?? "plugin";
    }

    public async Task GetManifestAsync()
    {
        await GetManifestAsync(new RestRequest(Method.GET));
    }

    public async Task GetRepoInfoAsync()
    {
        await GetRepoInfoAsync(new RestRequest(Method.GET));
    }

    public async Task GetRepoInfoAsync(RestRequest request)
    {
        var repoContent =
            JObject.Parse((await new RestClient($"https://api.github.com/repos/{RepoName}")
                .ExecuteAsync(request)).Content);
        Stars = repoContent.GetValue("watchers")?.ToObject<int>() ?? 0;
        AuthorLink = new Uri(((JObject)repoContent.GetValue("owner")).GetValue("html_url").ToString());
    }

    public async Task GetManifestAsync(RestRequest request)
    {
        var manifest =
            JObject.Parse(
                (await new RestClient(
                        $"https://raw.githubusercontent.com/{RepoName}/{RepoBranch}/manifest.json")
                    .ExecuteAsync(request)).Content);
        Slug = manifest.GetValue("slug")?.ToString() ?? Name.ToLower().Replace(' ', '-');
        Version = Version.TryParse(manifest.GetValue("version")?.ToString(), out Version version)
            ? version
            : new Version(1, 0, 0);
        AuthorLink = string.IsNullOrWhiteSpace(manifest.GetValue("author_link")?.ToString())
            ? new Uri(manifest.GetValue("author_link").ToString())
            : new Uri("https://github.com");
        Description = manifest.GetValue("description")?.ToString();
        Preview =
            $"https://raw.githubusercontent.com/{RepoName}/{RepoBranch}/{manifest.GetValue("preview")}";

        if (manifest.ContainsKey("requirements"))
        {
            Requirements.AddRange(manifest.GetValue("requirements")?.ToObject<List<string>>());
        }
        else
        {
            Requirements = new List<string>();
        }
    }

    public async Task InitializeAsync()
    {
        var request = new RestRequest(Method.GET);
        await Task.WhenAll(GetPluginInfoAsync(request), GetManifestAsync(request));
        IsInitialized = true;
    }
}