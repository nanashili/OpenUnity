using OpenUnity.Interop;

namespace UnityEngine;

public class Component : Object
{
    private readonly GameObject owner;

    public Component() : this(LifecycleDispatcher.ConsumeAttachment()) { }

    private Component(LifecycleDispatcher.Attachment attachment)
        : this(attachment.Context, attachment.Identity, attachment.Owner) { }

    internal Component(RuntimeContext context, long identity, GameObject owner)
        : base(context, identity) => this.owner = owner;

    internal GameObject AttachedOwner => owner;

    public GameObject gameObject
    {
        get
        {
            long found = Context.Invoke(runtime => OpenUnityNative.ou_component_gameobject(runtime, Identity));
            CheckNative(found, "component owner query");
            if (found != owner.Identity)
                throw new InvalidOperationException("OpenUnity returned an inconsistent component owner.");
            return owner;
        }
    }

    public Transform transform => gameObject.transform;

    public T GetComponent<T>() => gameObject.GetComponent<T>();

    public Component? GetComponent(Type type) => gameObject.GetComponent(type);

    public T[] GetComponents<T>() => gameObject.GetComponentsInternal(typeof(T))
        .Select(component => (T)(object)component).ToArray();
}
