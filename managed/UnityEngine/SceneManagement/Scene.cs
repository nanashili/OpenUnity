using System.Collections.Generic;
using System.Runtime.CompilerServices;
using OpenUnity.Interop;

namespace UnityEngine.SceneManagement;

public readonly struct Scene : IEquatable<Scene>
{
    private const int MaximumCopyAttempts = 5;
    private readonly RuntimeContext? context;
    private readonly long handle;

    private Scene(RuntimeContext context, long handle)
    {
        this.context = context;
        this.handle = handle;
    }

    public int rootCount
    {
        get
        {
            if (context is null)
                return 0;

            context.RequireSceneRoots();
            int count = context.Invoke(runtime =>
                OpenUnityNative.ou_scene_root_count(runtime, handle));
            if (count < 0)
                throw RuntimeContext.Failure("scene root count", count);
            return count;
        }
    }

    public bool IsValid()
    {
        if (context is null)
            return false;

        context.RequireSceneValidity();
        int status = context.Invoke(runtime => OpenUnityNative.ou_scene_is_valid(runtime, handle));
        if (status < 0)
        {
            if (status == -15)
                return false;

            throw RuntimeContext.Failure("scene validity query", status);
        }

        if (status > 1)
            throw new InvalidOperationException("OpenUnity returned an invalid scene validity result.");

        return status == 1;
    }

    public GameObject[] GetRootGameObjects()
    {
        if (context is null)
            return new GameObject[0];

        SceneRootIdentity[] identities = CaptureRootIdentities();
        GameObject[] roots = new GameObject[identities.Length];
        for (int index = 0; index < identities.Length; index++)
        {
            SceneRootIdentity identity = identities[index];
            roots[index] = GameObject.WrapSceneRoot(
                context,
                identity.GameObject,
                identity.Transform);
        }

        return roots;
    }

    public void GetRootGameObjects(List<GameObject> rootGameObjects)
    {
        ArgumentNullException.ThrowIfNull(rootGameObjects);
        GameObject[] roots = GetRootGameObjects();
        int requiredCapacity = checked(rootGameObjects.Count + roots.Length);
        rootGameObjects.EnsureCapacity(requiredCapacity);

        rootGameObjects.Clear();
        rootGameObjects.AddRange(roots);
    }

    public bool Equals(Scene other) =>
        ReferenceEquals(context, other.context) && handle == other.handle;

    public override bool Equals(object? obj) => obj is Scene other && Equals(other);

    public override int GetHashCode() => context is null
        ? 0
        : HashCode.Combine(RuntimeHelpers.GetHashCode(context), handle);

    public static bool operator ==(Scene left, Scene right) => left.Equals(right);

    public static bool operator !=(Scene left, Scene right) => !left.Equals(right);

    internal static Scene FromNativeHandle(RuntimeContext context, long handle) => handle switch
    {
        0 => default,
        > 0 => new Scene(context, handle),
        _ => throw RuntimeContext.Failure("GameObject scene query", handle)
    };

    private SceneRootIdentity[] CaptureRootIdentities()
    {
        RuntimeContext current = context!;
        current.RequireSceneRoots();

        for (int attempt = 0; attempt < MaximumCopyAttempts; attempt++)
        {
            int count = current.Invoke(runtime => OpenUnityNative.ou_scene_root_count(runtime, handle));
            if (count < 0)
                throw RuntimeContext.Failure("scene root count", count);

            SceneRootIdentity[] identities = new SceneRootIdentity[count];
            int copied = ReadRootIdentities(current, identities);
            if (copied == -7)
            {
                if (attempt + 1 < MaximumCopyAttempts)
                    continue;
                throw new InvalidOperationException(
                    "Scene roots changed repeatedly while the collection was captured.");
            }
            if (copied < 0)
                throw RuntimeContext.Failure("scene root copy", copied);
            if (copied > identities.Length)
                throw new InvalidOperationException("OpenUnity returned an invalid scene root count.");
            if (copied == identities.Length)
                return identities;

            return identities[..copied];
        }

        throw new InvalidOperationException("OpenUnity could not capture a stable scene root collection.");
    }

    private unsafe int ReadRootIdentities(
        RuntimeContext current,
        SceneRootIdentity[] identities) => current.Invoke(runtime =>
    {
        fixed (SceneRootIdentity* output = identities)
            return OpenUnityNative.ou_scene_roots_read(runtime, handle, output, identities.Length);
    });
}
