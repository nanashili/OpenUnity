using System.Text;
using OpenUnity.Interop;

namespace UnityEngine;

public sealed class GameObject : Object
{
    private static readonly object WrapperLock = new();
    private static readonly Dictionary<long, WeakReference<Object>> Wrappers = new();
    private readonly Transform ownedTransform;

    public GameObject() : this("New Game Object") { }

    public GameObject(string name) : this(Create(name)) { }

    private GameObject(Created created) : base(created.Context, created.Owner)
    {
        ownedTransform = new Transform(created.Context, created.Transform, this);
        lock (WrapperLock)
            Register(this);
        Context.RetainOwnerIfInitialized(this);
    }

    public Transform transform
    {
        get
        {
            long found = Context.Invoke(runtime => OpenUnityNative.ou_gameobject_transform(runtime, Identity));
            CheckNative(found, "GameObject Transform query");
            if (found != ownedTransform.Identity)
                throw new InvalidOperationException("OpenUnity returned an inconsistent mandatory Transform.");
            return ownedTransform;
        }
    }

    public T GetComponent<T>() => (T)(object?)GetComponent(typeof(T))!;

    public Component? GetComponent(Type type)
    {
        ArgumentNullException.ThrowIfNull(type);
        if (type == typeof(Transform))
            return transform;
        return GetComponentsInternal(type).FirstOrDefault();
    }

    public T AddComponent<T>() where T : Component => (T)AddComponent(typeof(T));

    public Component AddComponent(Type componentType) =>
        Context.Invoke(_ => Context.Lifecycle.AttachComponent(this, componentType));

    public bool activeSelf => Context.Lifecycle.ReadBoolean(runtime =>
        OpenUnityNative.ou_gameobject_active(runtime, Identity, 0), "activeSelf query");

    public bool activeInHierarchy => Context.Lifecycle.ReadBoolean(runtime =>
        OpenUnityNative.ou_gameobject_active(runtime, Identity, 1), "activeInHierarchy query");

    public void SetActive(bool value) => Context.Lifecycle.Mutate(runtime =>
        OpenUnityNative.ou_gameobject_set_active(runtime, Identity, value ? 1 : 0), "active state assignment");

    internal Component[] GetComponentsInternal(Type type) =>
        Context.Invoke(_ => Context.Lifecycle.Components(this, type));

    internal Transform SceneTransform => ownedTransform;

    internal static GameObject WrapSceneOwner(RuntimeContext context, long owner, long transform) =>
        new(new Created(context, owner, transform));

    internal static void ReleaseSceneWrapper(GameObject owner)
    {
        lock (WrapperLock)
        {
            foreach (long identity in new[] { owner.Identity, owner.ownedTransform.Identity })
                if (Wrappers.TryGetValue(identity, out WeakReference<Object>? weak) &&
                    weak.TryGetTarget(out Object? existing) &&
                    (ReferenceEquals(existing, owner) || ReferenceEquals(existing, owner.ownedTransform)))
                    Wrappers.Remove(identity);
        }
    }

    internal static Transform WrapTransform(RuntimeContext context, long identity)
    {
        CheckNative(identity, "Transform identity query");
        lock (WrapperLock)
        {
            if (Wrappers.TryGetValue(identity, out WeakReference<Object>? weak) &&
                weak.TryGetTarget(out Object? existing))
                return (Transform)existing;

            long owner = context.Invoke(runtime => OpenUnityNative.ou_component_gameobject(runtime, identity));
            CheckNative(owner, "Transform owner query");
            GameObject? retained = context.FindRetainedOwner(owner);
            if (!ReferenceEquals(retained, null))
                return retained.transform;
            if (Wrappers.TryGetValue(owner, out weak) && weak.TryGetTarget(out existing))
                return ((GameObject)existing).transform;

            long actual = context.Invoke(runtime => OpenUnityNative.ou_gameobject_transform(runtime, owner));
            CheckNative(actual, "GameObject Transform query");
            if (actual != identity)
                throw new InvalidOperationException("OpenUnity returned inconsistent Transform ownership.");
            return new GameObject(new Created(context, owner, identity)).ownedTransform;
        }
    }

    private static void Register(GameObject owner)
    {
        Wrappers[owner.Identity] = new WeakReference<Object>(owner);
        Wrappers[owner.ownedTransform.Identity] = new WeakReference<Object>(owner.ownedTransform);
    }

    private static Created Create(string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        byte[] bytes;
        try
        {
            UTF8Encoding utf8 = new(false, true);
            if (utf8.GetByteCount(name) > 1_048_576)
                throw new ArgumentException("OpenUnity names cannot exceed 1 MiB of UTF-8 bytes.", nameof(name));
            bytes = utf8.GetBytes(name);
        }
        catch (EncoderFallbackException error)
        {
            throw new ArgumentException("OpenUnity names must contain valid UTF-16.", nameof(name), error);
        }

        RuntimeContext context = RuntimeContext.Shared;
        context.RequireTransform();
        return context.Invoke(runtime =>
        {
            long owner = OpenUnityNative.ou_gameobject_create(runtime, bytes, bytes.Length);
            CheckNative(owner, "GameObject creation");
            if (owner == 0)
                throw RuntimeContext.Failure("GameObject creation", owner);
            try
            {
                _ = checked((int)owner);
                long transform = OpenUnityNative.ou_gameobject_transform(runtime, owner);
                CheckNative(transform, "GameObject Transform query");
                if (transform == 0)
                    throw RuntimeContext.Failure("GameObject Transform query", transform);
                _ = checked((int)transform);
                return new Created(context, owner, transform);
            }
            catch
            {
                int released = OpenUnityNative.ou_object_destroy(runtime, owner);
                if (released != 0)
                    throw RuntimeContext.Failure("failed GameObject creation cleanup", released);
                throw;
            }
        });
    }

    private readonly record struct Created(RuntimeContext Context, long Owner, long Transform);
}
