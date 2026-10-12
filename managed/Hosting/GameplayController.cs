using UnityEngine;

namespace OpenUnity.Hosting;

// The infrastructure controller implements the host protocol; ordinary user
// gameplay scripts inherit MonoBehaviour and receive native-selected events.
internal sealed class GameplayController : IHostedScript
{
    private readonly Type scriptType;
    private readonly LifecycleDispatcher dispatcher;
    private GameObject? root;

    internal GameplayController(Type scriptType)
    {
        LifecycleDispatcher.ValidateScriptType(scriptType);
        this.scriptType = scriptType;
        dispatcher = RuntimeContext.Shared.Lifecycle;
        dispatcher.CheckThread();
    }

    public long Initialize(long value)
    {
        root = new GameObject("OpenUnity Gameplay");
        root.AddComponent(scriptType);
        return 0;
    }

    public long Tick(long value) => dispatcher.Tick();

    public long Shutdown(long value)
    {
        UnityEngine.Object.Destroy(root);
        dispatcher.FlushDestroy();
        root = null;
        return 0;
    }
}
