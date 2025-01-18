using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Helpers;
using NonsPlayer.Models.Github;
using NonsPlayer.ViewModels;
using RestSharp;
using NonsPlayer.Core.Contracts.Plugins;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Views.Pages;

public sealed partial class AdapterManagerPage : Page
{
    private const string _repoURL = "https://api.github.com/repos/NonsPlayer/Nons-Plugins";
    private const string _cacheId = "plugin_models";
    public List<PluginModel> Plugins = new();

    public AdapterManagerPage()
    {
        ViewModel = App.GetService<AdapterManagerViewModel>();
        InitializeComponent();

        LoadPluginsFromRepo();
    }

    public AdapterManagerViewModel ViewModel { get; }

    private void LoadPluginsFromRepo()
    {
        CacheBase cacheBase;
        if (CacheHelper.Service.TryGet(_cacheId, out cacheBase))
        {
            if (cacheBase is { Value: not null })
            {
                Plugins = (List<PluginModel>)cacheBase.Value;
                foreach (PluginModel pluginModel in Plugins)
                {
                    PluginSwitcher(pluginModel);
                }

                return;
            }
        }

        var request = new RestRequest(Method.GET);

        Task.Run(async () =>
        {
            var response = await new RestClient(_repoURL).ExecuteAsync(request);
            if (response.IsSuccessful)
            {
                var repo = JObject.Parse(response.Content);

                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    ViewModel.UpdateTime = DateTime.Parse(repo.GetValue("updated_at").ToString()).ToString("g");
                });
            }
            else
            {
                var content = JObject.Parse(response.Content);
                ExceptionService.Instance.Throw(content.GetValue("message").ToString());
            }
        });

        Task.Run(async () =>
        {
            var plugins = (await new RestClient(_repoURL + "/contents/plugins").ExecuteAsync(request));
            if (plugins.IsSuccessful)
            {
                var content = JArray.Parse(plugins.Content);
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    ViewModel.PluginCount = content.Count.ToString();
                });
                foreach (JToken item in content)
                {
                    var path =
                        $"https://raw.githubusercontent.com/NonsPlayer/Nons-Plugins/refs/heads/master/{((JObject)item).GetValue("path")}";
                    var model = new PluginModel(path);
                    await model.GetPluginInfoAsync(request);
                    Plugins.Add(model);
                    CacheHelper.Service.AddOrUpdate("plugin_models", new CacheBase(Plugins));
                    ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                    {
                        PluginSwitcher(model);
                    });
                }
                    
            }
            else
            {
                var content = JObject.Parse(plugins.Content);
                ExceptionService.Instance.Throw(content.GetValue("message").ToString());
            }
        });
       
    }

    private void PluginSwitcher(PluginModel model)
    {
        switch (model.Type)
        {
            case "adapter":
                ViewModel.Adapters.Add(model);
                break;
            case "plugin":
                ViewModel.Plugins.Add(model);
                break;
            default:
                ViewModel.Plugins.Add(model);
                break;
        }
    }

    private void SelectorBar_OnSelectionChanged(SelectorBar sender, SelectorBarSelectionChangedEventArgs args)
    {
        switch (sender.SelectedItem.Tag.ToString())
        {
            case "adapter":
                ViewModel.PluginList.Clear();
                foreach (PluginModel item in ViewModel.Adapters)
                {
                    ViewModel.PluginList.Add(item);
                }

                break;
            case "plugin":
                ViewModel.PluginList.Clear();
                foreach (PluginModel item in ViewModel.Plugins)
                {
                    ViewModel.PluginList.Add(item);
                }

                break;
            case "installed":
                ViewModel.PluginList.Clear();
                ViewModel.Refresh();
                foreach (PluginModel item in ViewModel.Installed)
                {
                    ViewModel.PluginList.Add(item);
                }

                break;
        }
    }
}