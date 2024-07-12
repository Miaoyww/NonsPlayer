using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class AdapterModel
{
    public AdapterMetadata Metadata { get; set; }
    public int Index { get; set; }
}