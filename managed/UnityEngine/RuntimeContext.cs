using System.Runtime.InteropServices;
using OpenUnity.Interop;

namespace UnityEngine;

// The process context owns storage; individual wrappers cannot dispose it.
internal sealed class RuntimeContext : SafeHandle
{
    [ThreadStatic] private static bool constructingSceneScript;
    private static readonly Lazy<RuntimeContext> ProcessContext = new(Create);
    private static readonly Lazy<bool> TransformCapability = new(() =>
    {
        uint abi = OpenUnityNative.ou_transform_abi_version();
        if (abi != OpenUnityNative.TransformAbiVersion)
            throw new InvalidOperationException($"OpenUnity Transform ABI 1.0 is required; received 0x{abi:X8}.");
        return true;
    });
    private readonly long runtime;
    private readonly Lazy<LifecycleDispatcher> lifecycle;

    private RuntimeContext(long runtime) : base(IntPtr.Zero, ownsHandle: true)
    {
        this.runtime = runtime;
        lifecycle = new Lazy<LifecycleDispatcher>(() => new LifecycleDispatcher(this));
        // SafeHandle's marker is not the native identity or a storage pointer.
        SetHandle(new IntPtr(1));
    }

    internal static RuntimeContext Shared
    {
        get
        {
            RequireEngineAccess();
            return ProcessContext.Value;
        }
    }

    internal static void RequireEngineAccess()
    {
        if (constructingSceneScript)
            throw new NotSupportedException("OpenUnity scene constructors cannot access native engine state.");
    }

    internal static IDisposable SceneConstructorScope()
    {
        RequireEngineAccess();
        constructingSceneScript = true;
        return new ConstructorGuard();
    }

    private sealed class ConstructorGuard : IDisposable
    {
        public void Dispose() => constructingSceneScript = false;
    }

    internal void RequireTransform() => _ = TransformCapability.Value;

    internal LifecycleDispatcher Lifecycle => lifecycle.Value;

    internal void CheckLifecycleThreadIfInitialized()
    {
        if (lifecycle.IsValueCreated)
            lifecycle.Value.CheckThread();
    }

    internal void PumpLifecycleIfInitialized()
    {
        if (lifecycle.IsValueCreated)
            lifecycle.Value.PumpMutations();
    }

    internal void RetainOwnerIfInitialized(GameObject owner)
    {
        if (lifecycle.IsValueCreated)
            lifecycle.Value.RetainOwner(owner);
    }

    internal GameObject? FindRetainedOwner(long identity) => lifecycle.IsValueCreated
        ? lifecycle.Value.FindOwner(identity) : null;

    public override bool IsInvalid => handle == IntPtr.Zero;

    internal T Invoke<T>(Func<long, T> operation)
    {
        RequireEngineAccess();
        bool retained = false;
        try
        {
            DangerousAddRef(ref retained);
            return operation(runtime);
        }
        finally
        {
            if (retained)
                DangerousRelease();
        }
    }

    internal static InvalidOperationException Failure(string operation, long status) =>
        new($"OpenUnity {operation} failed with native status {status}.");

    protected override bool ReleaseHandle()
    {
        try
        {
            return OpenUnityNative.ou_runtime_destroy(runtime) == 0;
        }
        catch
        {
            // Finalization must not throw; shutdown finalization is not guaranteed.
            return false;
        }
    }

    private static RuntimeContext Create()
    {
        uint abi = OpenUnityNative.ou_abi_version();
        if (abi != OpenUnityNative.AbiVersion)
            throw new InvalidOperationException($"OpenUnity ABI 1.0 is required; received 0x{abi:X8}.");
        uint nameAbi = OpenUnityNative.ou_object_name_abi_version();
        if (nameAbi != OpenUnityNative.ObjectNameAbiVersion)
            throw new InvalidOperationException($"OpenUnity object-name ABI 1.0 is required; received 0x{nameAbi:X8}.");

        long identity = OpenUnityNative.ou_runtime_create();
        if (identity <= 0)
            throw Failure("runtime creation", identity);
        return new RuntimeContext(identity);
    }
}
