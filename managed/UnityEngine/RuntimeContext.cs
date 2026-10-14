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
    private static readonly Lazy<bool> TransformHierarchyCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_transform_hierarchy_abi_version();
            if (abi != OpenUnityNative.TransformHierarchyAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Transform hierarchy ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Transform hierarchy ABI 1.0 is unavailable.", error);
        }
    });
    private static readonly Lazy<bool> TransformDirectionCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_transform_direction_abi_version();
            if (abi != OpenUnityNative.TransformDirectionAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Transform direction ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Transform direction ABI 1.0 is unavailable.", error);
        }
    });
    private static readonly Lazy<bool> TransformDetachCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_transform_detach_abi_version();
            if (abi != OpenUnityNative.TransformDetachAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Transform detach ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Transform detach ABI 1.0 is unavailable.", error);
        }
    });
    private static readonly Lazy<bool> TransformVectorCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_transform_vector_abi_version();
            if (abi != OpenUnityNative.TransformVectorAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Transform vector ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Transform vector ABI 1.0 is unavailable.", error);
        }
    });
    private readonly long runtime;
    private Exception? unusableReason;
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
            RuntimeContext context = ProcessContext.Value;
            context.RequireUsable();
            return context;
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

    internal void RequireTransformHierarchy() => _ = TransformHierarchyCapability.Value;

    internal void RequireTransformDirection() => _ = TransformDirectionCapability.Value;

    internal void RequireTransformDetach() => _ = TransformDetachCapability.Value;

    internal void RequireTransformVector() => _ = TransformVectorCapability.Value;

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

    internal void RejectFurtherUse(Exception failure)
    {
        unusableReason ??= failure;
    }

    private void RequireUsable()
    {
        if (unusableReason is not null)
            throw new InvalidOperationException(
                "OpenUnity context cannot be reused after failed initialization or ownership recovery.", unusableReason);
    }

    internal T Invoke<T>(Func<long, T> operation)
    {
        RequireUsable();
        return InvokeCleanup(operation);
    }

    // Only ownership release may bypass a failed timed initialization guard.
    // This cannot restart a scene, advance time, or publish callbacks.
    internal T InvokeCleanup<T>(Func<long, T> operation)
    {
        RequireEngineAccess();
        CheckLifecycleThreadIfInitialized();
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
