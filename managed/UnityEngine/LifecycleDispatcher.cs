using System.Reflection;
using OpenUnity.Interop;

namespace UnityEngine;

// Native state selects every event. This adapter only binds constructors,
// retains canonical wrappers and invokes the returned callback snapshots.
internal sealed partial class LifecycleDispatcher
{
    private static readonly IReadOnlyDictionary<int, string> EventNames = new Dictionary<int, string>
    {
        [1] = "Awake",
        [2] = "OnEnable",
        [3] = "Start",
        [4] = "Update",
        [5] = "LateUpdate",
        [6] = "OnDisable",
        [7] = "OnDestroy",
        [9] = "OnTransformChildrenChanged",
        [10] = "OnTransformParentChanged"
    };
    [ThreadStatic] private static ConstructionScope? construction;
    private readonly RuntimeContext context;
    private readonly int ownerThread = Environment.CurrentManagedThreadId;
    private readonly Dictionary<long, MonoBehaviour> attachments = new();
    private readonly Dictionary<long, GameObject> owners = new();
    private readonly Dictionary<Type, IReadOnlyDictionary<int, MethodInfo?>> callbacks = new();
    private int pumpDepth;
    private bool timed;
    private readonly Dictionary<Type, MethodInfo?> fixedCallbacks = new();
    private readonly List<ComponentReservation> failedReservations = new();

    internal LifecycleDispatcher(RuntimeContext context)
    {
        uint abi = OpenUnityNative.ou_lifecycle_abi_version();
        if (abi != OpenUnityNative.LifecycleAbiVersion)
            throw new InvalidOperationException($"OpenUnity lifecycle ABI 2.0 is required; received 0x{abi:X8}.");
        this.context = context;
    }

    internal bool IsOwnerThread => Environment.CurrentManagedThreadId == ownerThread;

    internal void CheckThread()
    {
        RuntimeContext.RequireEngineAccess();
        if (!IsOwnerThread)
            throw new NotSupportedException("OpenUnity lifecycle operations must run on their context's owner thread.");
    }

    internal bool ReadBoolean(Func<long, int> operation, string name)
    {
        CheckThread();
        int result = context.Invoke(operation);
        Object.CheckNative(result, name);
        return result switch
        {
            0 => false,
            1 => true,
            _ => throw RuntimeContext.Failure(name, result)
        };
    }

    internal void Mutate(Func<long, int> operation, string name)
    {
        CheckThread();
        RequireSuccess(context.Invoke(operation), name);
        PumpMutations();
    }

    private static void RequireSuccess(int status, string operation)
    {
        Object.CheckNative(status, operation);
        if (status != 0)
            throw RuntimeContext.Failure(operation, status);
    }
}
