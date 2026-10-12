using OpenUnity.Interop;

namespace UnityEngine;

public class Behaviour : Component
{
    public Behaviour() { }

    public bool enabled
    {
        get => Context.Lifecycle.ReadBoolean(runtime =>
            OpenUnityNative.ou_behaviour_enabled(runtime, Identity), "enabled query");
        set => Context.Lifecycle.Mutate(runtime =>
            OpenUnityNative.ou_behaviour_set_enabled(runtime, Identity, value ? 1 : 0), "enabled assignment");
    }

    public bool isActiveAndEnabled => Context.Lifecycle.ReadBoolean(runtime =>
        OpenUnityNative.ou_behaviour_active(runtime, Identity), "behaviour activity query");
}
